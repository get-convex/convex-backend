import chalk from "chalk";
import util from "util";
import ws from "ws";
import { ConvexHttpClient } from "../../browser/http_client.js";
import { BaseConvexClient } from "../../browser/index.js";
import { PaginationResult, makeFunctionReference } from "../../server/index.js";
import { Value, convexToJson } from "../../values/value.js";
import {
  Context,
  logError,
  logFailure,
  logFinishedStep,
  logMessage,
  logOutput,
} from "../../bundler/context.js";
import { waitForever, waitUntilCalled } from "./utils.js";

export async function runFunctionAndLog(
  ctx: Context,
  deploymentUrl: string,
  adminKey: string,
  functionName: string,
  args: Value,
  componentPath?: string,
  callbacks?: {
    onSuccess?: () => void;
  },
) {
  const client = new ConvexHttpClient(deploymentUrl);
  client.setAdminAuth(adminKey);

  let result: Value;
  try {
    result = await client.function(
      makeFunctionReference(functionName),
      componentPath,
      args,
    );
  } catch (err) {
    logFailure(ctx, `Failed to run function "${functionName}":`);
    logError(ctx, chalk.red((err as Error).toString().trim()));
    return await ctx.crash(1, "invalid filesystem or env vars");
  }

  callbacks?.onSuccess?.();

  // `null` is the default return type
  if (result !== null) {
    logOutput(ctx, formatValue(result));
  }
}

export async function runPaginatedQuery(
  ctx: Context,
  deploymentUrl: string,
  adminKey: string,
  functionName: string,
  args: Record<string, Value>,
  limit?: number,
) {
  const results = [];
  let cursor = null;
  let isDone = false;
  while (!isDone && (limit === undefined || results.length < limit)) {
    const paginationResult = (await runQuery(
      ctx,
      deploymentUrl,
      adminKey,
      functionName,
      {
        ...args,
        // The pagination is limited on the backend, so the 10000
        // means "give me as many as possible".
        paginationOpts: {
          cursor,
          numItems: limit === undefined ? 10000 : limit - results.length,
        },
      },
    )) as unknown as PaginationResult<Record<string, Value>>;
    isDone = paginationResult.isDone;
    cursor = paginationResult.continueCursor;
    results.push(...paginationResult.page);
  }
  return results;
}

export async function runQuery(
  ctx: Context,
  deploymentUrl: string,
  adminKey: string,
  functionName: string,
  args: Record<string, Value>,
): Promise<Value> {
  const client = new ConvexHttpClient(deploymentUrl);
  client.setAdminAuth(adminKey);

  try {
    return await client.query(
      makeFunctionReference<"query">(functionName),
      args,
    );
  } catch (err) {
    logFailure(ctx, `Failed to run query "${functionName}":`);
    logError(ctx, chalk.red((err as Error).toString().trim()));
    return await ctx.crash(1, "invalid filesystem or env vars");
  }
}

export function formatValue(value: Value) {
  const json = convexToJson(value);
  if (process.stdout.isTTY) {
    // TODO (Tom) add JSON syntax highlighting like https://stackoverflow.com/a/51319962/398212
    // until then, just spit out something that isn't quite JSON because it's easy
    return util.inspect(value, { colors: true, depth: null });
  } else {
    return JSON.stringify(json, null, 2);
  }
}

export async function subscribeAndLog(
  ctx: Context,
  deploymentUrl: string,
  adminKey: string,
  functionName: string,
  args: Record<string, Value>,
) {
  return subscribe(
    ctx,
    deploymentUrl,
    adminKey,
    functionName,
    args,
    waitForever(),
    {
      onStart() {
        logFinishedStep(
          ctx,
          `Watching query ${functionName} on ${deploymentUrl}...`,
        );
      },
      onChange(result) {
        logOutput(ctx, formatValue(result));
      },
      onStop() {
        logMessage(ctx, `Closing connection to ${deploymentUrl}...`);
      },
    },
  );
}

export async function subscribe(
  ctx: Context,
  deploymentUrl: string,
  adminKey: string,
  functionName: string,
  args: Record<string, Value>,
  until: Promise<unknown>,
  callbacks?: {
    onStart?: () => void;
    onChange?: (result: Value) => void;
    onStop?: () => void;
  },
) {
  const client = new BaseConvexClient(
    deploymentUrl,
    (updatedQueries) => {
      for (const queryToken of updatedQueries) {
        callbacks?.onChange?.(client.localQueryResultByToken(queryToken)!);
      }
    },
    {
      // pretend that a Node.js 'ws' library WebSocket is a browser WebSocket
      webSocketConstructor: ws as unknown as typeof WebSocket,
      unsavedChangesWarning: false,
    },
  );
  client.setAdminAuth(adminKey);
  const { unsubscribe } = client.subscribe(functionName, args);

  callbacks?.onStart?.();

  let done = false;
  const [donePromise, onDone] = waitUntilCalled();
  const stopWatching = () => {
    if (done) {
      return;
    }
    done = true;
    unsubscribe();
    void client.close();
    process.off("SIGINT", sigintListener);
    onDone();
    callbacks?.onStop?.();
  };
  function sigintListener() {
    stopWatching();
  }
  process.on("SIGINT", sigintListener);
  void until.finally(stopWatching);
  while (!done) {
    // loops once per day (any large value < 2**31 would work)
    const oneDay = 24 * 60 * 60 * 1000;
    await Promise.race([
      donePromise,
      new Promise((resolve) => setTimeout(resolve, oneDay)),
    ]);
  }
}
