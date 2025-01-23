import {
  changeSpinner,
  Context,
  logError,
  logFailure,
  logVerbose,
} from "../../bundler/context.js";
import {
  deploymentFetch,
  ErrorData,
  logAndHandleFetchError,
  ThrowingFetchError,
} from "./utils/utils.js";
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
import { getTargetDeploymentName } from "./deployment.js";
import { deploymentDashboardUrlPage } from "../dashboard.js";
import { finishPushDiff, FinishPushDiff } from "./deployApi/finishPush.js";
import { Reporter, Span } from "./tracing.js";
import { promisify } from "node:util";
import zlib from "node:zlib";

const brotli = promisify(zlib.brotliCompress);

async function brotliCompress(ctx: Context, data: string): Promise<Buffer> {
  const start = performance.now();
  const result = await brotli(data, {
    params: {
      [zlib.constants.BROTLI_PARAM_MODE]: zlib.constants.BROTLI_MODE_TEXT,
      [zlib.constants.BROTLI_PARAM_QUALITY]: 4,
    },
  });
  const end = performance.now();
  const duration = end - start;
  logVerbose(
    ctx,
    `Compressed ${(data.length / 1024).toFixed(2)}KiB to ${(result.length / 1024).toFixed(2)}KiB (${((result.length / data.length) * 100).toFixed(2)}%) in ${duration.toFixed(2)}ms`,
  );
  return result;
}

/** Push configuration2 to the given remote origin. */
export async function startPush(
  ctx: Context,
  span: Span,
  request: StartPushRequest,
  options: {
    url: string;
  },
): Promise<StartPushResponse> {
  const custom = (_k: string | number, s: any) =>
    typeof s === "string" ? s.slice(0, 40) + (s.length > 40 ? "..." : "") : s;
  logVerbose(ctx, JSON.stringify(request, custom, 2));
  const onError = (err: any) => {
    if (err.toString() === "TypeError: fetch failed") {
      changeSpinner(
        ctx,
        `Fetch failed, is ${options.url} correct? Retrying...`,
      );
    }
  };
  const fetch = deploymentFetch(ctx, {
    deploymentUrl: options.url,
    adminKey: request.adminKey,
    onError,
  });
  changeSpinner(ctx, "Analyzing source code...");
  try {
    const response = await fetch("/api/deploy2/start_push", {
      body: await brotliCompress(ctx, JSON.stringify(request)),
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Content-Encoding": "br",
        traceparent: span.encodeW3CTraceparent(),
      },
    });
    return startPushResponse.parse(await response.json());
  } catch (error: unknown) {
    const data: ErrorData | undefined =
      error instanceof ThrowingFetchError ? error.serverErrorData : undefined;
    if (data?.code === "AuthConfigMissingEnvironmentVariable") {
      const errorMessage = data.message || "(no error message given)";
      // If `npx convex dev` is running using --url there might not be a configured deployment
      const configuredDeployment = getTargetDeploymentName();
      const [, variableName] =
        errorMessage.match(/Environment variable (\S+)/i) ?? [];
      const variableQuery =
        variableName !== undefined ? `?var=${variableName}` : "";
      const dashboardUrl = deploymentDashboardUrlPage(
        configuredDeployment,
        `/settings/environment-variables${variableQuery}`,
      );
      const message =
        `Environment variable ${chalk.bold(
          variableName,
        )} is used in auth config file but ` +
        `its value was not set. Go to:\n\n    ${chalk.bold(
          dashboardUrl,
        )}\n\n  to set it up. `;
      await ctx.crash({
        exitCode: 1,
        errorType: "invalid filesystem or env vars",
        errForSentry: error,
        printedMessage: message,
      });
    }
    logFailure(ctx, "Error: Unable to start push to " + options.url);
    return await logAndHandleFetchError(ctx, error);
  }
}

// Long poll every 10s for progress on schema validation.
const SCHEMA_TIMEOUT_MS = 10_000;

export async function waitForSchema(
  ctx: Context,
  span: Span,
  startPush: StartPushResponse,
  options: {
    adminKey: string;
    url: string;
    dryRun: boolean;
  },
) {
  const fetch = deploymentFetch(ctx, {
    deploymentUrl: options.url,
    adminKey: options.adminKey,
  });

  changeSpinner(
    ctx,
    "Backfilling indexes and checking that documents match your schema...",
  );

  while (true) {
    let currentStatus: SchemaStatus;
    try {
      const response = await fetch("/api/deploy2/wait_for_schema", {
        body: JSON.stringify({
          adminKey: options.adminKey,
          schemaChange: startPush.schemaChange,
          timeoutMs: SCHEMA_TIMEOUT_MS,
          dryRun: options.dryRun,
        }),
        method: "POST",
        headers: {
          traceparent: span.encodeW3CTraceparent(),
        },
      });
      currentStatus = schemaStatus.parse(await response.json());
    } catch (error: unknown) {
      logFailure(ctx, "Error: Unable to wait for schema from " + options.url);
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
            "invalid filesystem or db data": currentStatus.tableName
              ? {
                  tableName: currentStatus.tableName,
                  componentPath: currentStatus.componentPath,
                }
              : null,
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
  span: Span,
  startPush: StartPushResponse,
  options: {
    adminKey: string;
    url: string;
    dryRun: boolean;
    verbose?: boolean;
  },
): Promise<FinishPushDiff> {
  changeSpinner(ctx, "Finalizing push...");
  const fetch = deploymentFetch(ctx, {
    deploymentUrl: options.url,
    adminKey: options.adminKey,
  });
  const request = {
    adminKey: options.adminKey,
    startPush,
    dryRun: options.dryRun,
  };
  try {
    const response = await fetch("/api/deploy2/finish_push", {
      body: await brotliCompress(ctx, JSON.stringify(request)),
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Content-Encoding": "br",
        traceparent: span.encodeW3CTraceparent(),
      },
    });
    return finishPushDiff.parse(await response.json());
  } catch (error: unknown) {
    logFailure(ctx, "Error: Unable to finish push to " + options.url);
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

export async function reportPushCompleted(
  ctx: Context,
  adminKey: string,
  url: string,
  reporter: Reporter,
) {
  const fetch = deploymentFetch(ctx, {
    deploymentUrl: url,
    adminKey,
  });
  try {
    const response = await fetch("/api/deploy2/report_push_completed", {
      body: JSON.stringify({
        adminKey,
        spans: reporter.spans,
      }),
      method: "POST",
    });
    await response.json();
  } catch (error: unknown) {
    logFailure(
      ctx,
      "Error: Unable to report push completed to " + url + ": " + error,
    );
  }
}
