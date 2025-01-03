import chalk from "chalk";
import util from "util";
import ws from "ws";
import { ConvexHttpClient } from "../../browser/http_client.js";
import { BaseConvexClient } from "../../browser/index.js";
import {
  PaginationResult,
  UserIdentityAttributes,
  makeFunctionReference,
} from "../../server/index.js";
import { Value, convexToJson, jsonToConvex } from "../../values/value.js";
import {
  Context,
  logFinishedStep,
  logMessage,
  logOutput,
} from "../../bundler/context.js";
import { waitForever, waitUntilCalled } from "./utils/utils.js";
import JSON5 from "json5";
import path from "path";
import { readProjectConfig } from "./config.js";

export async function runFunctionAndLog(
  ctx: Context,
  args: {
    deploymentUrl: string;
    adminKey: string;
    functionName: string;
    argsString: string;
    identityString?: string;
    componentPath?: string;
    callbacks?: {
      onSuccess?: () => void;
    };
  },
) {
  const client = new ConvexHttpClient(args.deploymentUrl);
  const identity = args.identityString
    ? await getFakeIdentity(ctx, args.identityString)
    : undefined;
  client.setAdminAuth(args.adminKey, identity);

  const functionArgs = await parseArgs(ctx, args.argsString);
  const { projectConfig } = await readProjectConfig(ctx);
  const parsedFunctionName = await parseFunctionName(
    ctx,
    args.functionName,
    projectConfig.functions,
  );
  let result: Value;
  try {
    result = await client.function(
      makeFunctionReference(parsedFunctionName),
      args.componentPath,
      functionArgs,
    );
  } catch (err) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem or env vars",
      printedMessage: `Failed to run function "${args.functionName}":\n${chalk.red((err as Error).toString().trim())}`,
    });
  }

  args.callbacks?.onSuccess?.();

  // `null` is the default return type
  if (result !== null) {
    logOutput(ctx, formatValue(result));
  }
}

async function getFakeIdentity(ctx: Context, identityString: string) {
  let identity: UserIdentityAttributes;
  try {
    identity = JSON5.parse(identityString);
  } catch (err) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Failed to parse identity as JSON: "${identityString}"\n${chalk.red((err as Error).toString().trim())}`,
    });
  }
  const subject = identity.subject ?? "" + simpleHash(JSON.stringify(identity));
  const issuer = identity.issuer ?? "https://convex.test";
  const tokenIdentifier =
    identity.tokenIdentifier ?? `${issuer.toString()}|${subject.toString()}`;
  return {
    ...identity,
    subject,
    issuer,
    tokenIdentifier,
  };
}

async function parseArgs(ctx: Context, argsString: string) {
  try {
    const argsJson = JSON5.parse(argsString);
    return jsonToConvex(argsJson) as Record<string, Value>;
  } catch (err) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem or env vars",
      printedMessage: `Failed to parse arguments as JSON: "${argsString}"\n${chalk.red((err as Error).toString().trim())}`,
    });
  }
}

export async function parseFunctionName(
  ctx: Context,
  functionName: string,
  // Usually `convex/` -- should contain trailing slash
  functionDirName: string,
) {
  // api.foo.bar -> foo:bar
  // foo/bar -> foo/bar:default
  // foo/bar:baz -> foo/bar:baz
  // convex/foo/bar -> foo/bar:default

  // This is the `api.foo.bar` format
  if (functionName.startsWith("api.") || functionName.startsWith("internal.")) {
    const parts = functionName.split(".");
    if (parts.length < 3) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Function name has too few parts: "${functionName}"`,
      });
    }
    const exportName = parts.pop();
    const parsedName = `${parts.slice(1).join("/")}:${exportName}`;
    return parsedName;
  }

  // This is the `foo/bar:baz` format

  // This is something like `convex/foo/bar`, which could either be addressing `foo/bar:default` or `convex/foo/bar:default`
  // if there's a directory with the same name as the functions directory nested directly underneath.
  // We'll prefer the `convex/foo/bar:default` version, and check if the file exists, and otherwise treat this as a relative path from the project root.
  const filePath = functionName.split(":")[0];
  const exportName = functionName.split(":")[1] ?? "default";
  const normalizedName = `${filePath}:${exportName}`;

  // This isn't a relative path from the project root
  if (!filePath.startsWith(functionDirName)) {
    return normalizedName;
  }

  const filePathWithoutPrefix = filePath.slice(functionDirName.length);
  const functionNameWithoutPrefix = `${filePathWithoutPrefix}:${exportName}`;

  const possibleExtensions = [".ts", ".js", ".tsx", ".jsx"];
  const hasExtension = possibleExtensions.some((extension) =>
    filePath.endsWith(extension),
  );
  if (hasExtension) {
    if (ctx.fs.exists(path.join(functionDirName, filePath))) {
      return normalizedName;
    } else {
      return functionNameWithoutPrefix;
    }
  } else {
    const exists = possibleExtensions.some((extension) =>
      ctx.fs.exists(path.join(functionDirName, filePath + extension)),
    );
    if (exists) {
      return normalizedName;
    } else {
      return functionNameWithoutPrefix;
    }
  }
}

function simpleHash(string: string) {
  let hash = 0;
  for (let i = 0; i < string.length; i++) {
    const char = string.charCodeAt(i);
    hash = (hash << 5) - hash + char;
    hash = hash & hash; // Convert to 32bit integer
  }
  return hash;
}

export async function runSystemPaginatedQuery(
  ctx: Context,
  args: {
    deploymentUrl: string;
    adminKey: string;
    functionName: string;
    componentPath: string | undefined;
    args: Record<string, Value>;
    limit?: number;
  },
) {
  const results = [];
  let cursor = null;
  let isDone = false;
  while (!isDone && (args.limit === undefined || results.length < args.limit)) {
    const paginationResult = (await runSystemQuery(ctx, {
      ...args,
      args: {
        ...args.args,
        paginationOpts: {
          cursor,
          numItems:
            args.limit === undefined ? 10000 : args.limit - results.length,
        },
      },
    })) as unknown as PaginationResult<Record<string, Value>>;
    isDone = paginationResult.isDone;
    cursor = paginationResult.continueCursor;
    results.push(...paginationResult.page);
  }
  return results;
}

export async function runSystemQuery(
  ctx: Context,
  args: {
    deploymentUrl: string;
    adminKey: string;
    functionName: string;
    componentPath: string | undefined;
    args: Record<string, Value>;
  },
): Promise<Value> {
  let onResult: (result: Value) => void;
  const resultPromise = new Promise<Value>((resolve) => {
    onResult = resolve;
  });
  const [donePromise, onDone] = waitUntilCalled();
  await subscribe(ctx, {
    ...args,
    parsedFunctionName: args.functionName,
    parsedFunctionArgs: args.args,
    until: donePromise,
    callbacks: {
      onChange: (result) => {
        onDone();
        onResult(result);
      },
    },
  });
  return resultPromise;
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
  args: {
    deploymentUrl: string;
    adminKey: string;
    functionName: string;
    argsString: string;
    identityString?: string;
    componentPath: string | undefined;
  },
) {
  const { projectConfig } = await readProjectConfig(ctx);

  const parsedFunctionName = await parseFunctionName(
    ctx,
    args.functionName,
    projectConfig.functions,
  );
  const identity = args.identityString
    ? await getFakeIdentity(ctx, args.identityString)
    : undefined;
  const functionArgs = await parseArgs(ctx, args.argsString);
  return subscribe(ctx, {
    deploymentUrl: args.deploymentUrl,
    adminKey: args.adminKey,
    identity,
    parsedFunctionName,
    parsedFunctionArgs: functionArgs,
    componentPath: args.componentPath,
    until: waitForever(),
    callbacks: {
      onStart() {
        logFinishedStep(
          ctx,
          `Watching query ${args.functionName} on ${args.deploymentUrl}...`,
        );
      },
      onChange(result) {
        logOutput(ctx, formatValue(result));
      },
      onStop() {
        logMessage(ctx, `Closing connection to ${args.deploymentUrl}...`);
      },
    },
  });
}

export async function subscribe(
  ctx: Context,
  args: {
    deploymentUrl: string;
    adminKey: string;
    identity?: UserIdentityAttributes;
    parsedFunctionName: string;
    parsedFunctionArgs: Record<string, Value>;
    componentPath: string | undefined;
    until: Promise<unknown>;
    callbacks?: {
      onStart?: () => void;
      onChange?: (result: Value) => void;
      onStop?: () => void;
    };
  },
) {
  const client = new BaseConvexClient(
    args.deploymentUrl,
    (updatedQueries) => {
      for (const queryToken of updatedQueries) {
        args.callbacks?.onChange?.(client.localQueryResultByToken(queryToken)!);
      }
    },
    {
      // pretend that a Node.js 'ws' library WebSocket is a browser WebSocket
      webSocketConstructor: ws as unknown as typeof WebSocket,
      unsavedChangesWarning: false,
    },
  );
  client.setAdminAuth(args.adminKey, args.identity);
  const { unsubscribe } = client.subscribe(
    args.parsedFunctionName,
    args.parsedFunctionArgs,
    {
      componentPath: args.componentPath,
    },
  );

  args.callbacks?.onStart?.();

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
    args.callbacks?.onStop?.();
  };
  function sigintListener() {
    stopWatching();
  }
  process.on("SIGINT", sigintListener);
  void args.until.finally(stopWatching);
  while (!done) {
    // loops once per day (any large value < 2**31 would work)
    const oneDay = 24 * 60 * 60 * 1000;
    await Promise.race([
      donePromise,
      new Promise((resolve) => setTimeout(resolve, oneDay)),
    ]);
  }
}
