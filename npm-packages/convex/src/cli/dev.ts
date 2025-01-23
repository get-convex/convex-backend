import chalk from "chalk";
import { Command, Option } from "@commander-js/extra-typings";
import path from "path";
import { performance } from "perf_hooks";
import {
  OneoffCtx,
  logError,
  logFinishedStep,
  logMessage,
  logOutput,
  logVerbose,
  logWarning,
  oneoffContext,
  showSpinner,
  showSpinnerIfSlow,
  stopSpinner,
} from "../bundler/context.js";
import { deploymentCredentialsOrConfigure } from "./configure.js";
import { checkAuthorization, performLogin } from "./lib/login.js";
import { PushOptions } from "./lib/push.js";
import {
  formatDuration,
  getCurrentTimeString,
  waitForever,
  waitUntilCalled,
} from "./lib/utils/utils.js";
import { Crash, WatchContext, Watcher } from "./lib/watch.js";
import { watchLogs } from "./lib/logs.js";
import { runFunctionAndLog, subscribe } from "./lib/run.js";
import { Value } from "../values/index.js";
import { usageStateWarning } from "./lib/usage.js";
import { runPush } from "./lib/components.js";
import {
  bigBrainEnableFeatureMetadata,
  projectHasExistingDev,
} from "./lib/localDeployment/bigBrain.js";

export const dev = new Command("dev")
  .summary("Develop against a dev deployment, watching for changes")
  .description(
    "Develop against a dev deployment, watching for changes\n\n" +
      "  1. Configures a new or existing project (if needed)\n" +
      "  2. Updates generated types and pushes code to the configured dev deployment\n" +
      "  3. Runs the provided function (if `--run` is used)\n" +
      "  4. Watches for file changes, and repeats step 2\n",
  )
  .allowExcessArguments(false)
  .option("-v, --verbose", "Show full listing of changes")
  .addOption(
    new Option(
      "--typecheck <mode>",
      `Check TypeScript files with \`tsc --noEmit\`.`,
    )
      .choices(["enable", "try", "disable"] as const)
      .default("try" as const),
  )
  .option(
    "--typecheck-components",
    "Check TypeScript files within component implementations with `tsc --noEmit`.",
    false,
  )
  .addOption(
    new Option("--codegen <mode>", "Regenerate code in `convex/_generated/`")
      .choices(["enable", "disable"] as const)
      .default("enable" as const),
  )
  .addOption(
    new Option(
      "--configure [choice]",
      "Ignore existing configuration and configure new or existing project, interactively or set by --team <team_slug> and --project <project_slug>",
    ).choices(["new", "existing"] as const),
  )
  .addOption(
    new Option(
      "--team <team_slug>",
      "The team you'd like to use for this project",
    ).hideHelp(),
  )
  .addOption(
    new Option(
      "--project <project_slug>",
      "The name of the project you'd like to configure",
    ).hideHelp(),
  )
  .option(
    "--once",
    "Execute only the first 3 steps, stop on any failure",
    false,
  )
  .option(
    "--until-success",
    "Execute only the first 3 steps, on failure watch for local and remote changes and retry steps 2 and 3",
    false,
  )
  .option(
    "--run <functionName>",
    "The identifier of the function to run in step 3, " +
      "like `init` or `dir/file:myFunction`",
  )
  .option(
    "--run-component <functionName>",
    "If --run is used and the function is in a component, the path the component tree defined in convex.config.ts. " +
      "Components are a beta feature. This flag is unstable and may change in subsequent releases.",
  )
  .addOption(
    new Option(
      "--prod",
      "Develop live against this project's production deployment.",
    )
      .default(false)
      .hideHelp(),
  )
  .addOption(
    new Option(
      "--tail-logs",
      "Tail this project's Convex logs in this terminal.",
    ),
  )
  .addOption(new Option("--trace-events").default(false).hideHelp())
  .addOption(new Option("--verbose").default(false).hideHelp())
  .addOption(new Option("--skip-push").default(false).hideHelp())
  .addOption(new Option("--admin-key <adminKey>").hideHelp())
  .addOption(new Option("--url <url>").hideHelp())
  .addOption(new Option("--debug-bundle-path <path>").hideHelp())
  // Options for testing
  .addOption(new Option("--override-auth-url <url>").hideHelp())
  .addOption(new Option("--override-auth-client <id>").hideHelp())
  .addOption(new Option("--override-auth-username <username>").hideHelp())
  .addOption(new Option("--override-auth-password <password>").hideHelp())
  .addOption(
    new Option("--local", "Develop live against a locally running backend.")
      .default(false)
      .conflicts(["--prod", "--url", "--admin-key"])
      .hideHelp(),
  )
  .addOption(new Option("--local-cloud-port <port>").hideHelp())
  .addOption(new Option("--local-site-port <port>").hideHelp())
  .addOption(new Option("--local-backend-version <version>").hideHelp())
  .addOption(new Option("--local-force-upgrade").default(false).hideHelp())
  .addOption(new Option("--live-component-sources").hideHelp())
  .addOption(new Option("--enable-feature-metadata").hideHelp())
  .addOption(new Option("--has-existing-dev").hideHelp())
  .addOption(new Option("--partition-id <id>").hideHelp())
  .showHelpAfterError()
  .action(async (cmdOptions) => {
    const ctx = oneoffContext();
    process.on("SIGINT", async () => {
      logVerbose(ctx, "Received SIGINT, cleaning up...");
      await ctx.flushAndExit(-2);
    });

    // These two commands are temporary for testing, until they're hooked up, see ENG-8285
    if (cmdOptions.enableFeatureMetadata) {
      const enableFeatureMetadata = await bigBrainEnableFeatureMetadata(ctx);
      logOutput(ctx, enableFeatureMetadata);
      await ctx.flushAndExit(0);
    }

    if (cmdOptions.hasExistingDev) {
      if (!cmdOptions.team || !cmdOptions.project) {
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage: "Must specify --team and --project",
        });
      }
      const hasExistingDev = await projectHasExistingDev(ctx, {
        teamSlug: cmdOptions.team,
        projectSlug: cmdOptions.project,
      });
      logOutput(ctx, hasExistingDev);
      await ctx.flushAndExit(0);
    }
    /** End temp testing options */

    if (cmdOptions.runComponent && !cmdOptions.run) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage:
          "Can't specify `--run-component` option without `--run`",
      });
    }

    if (cmdOptions.debugBundlePath !== undefined && !cmdOptions.once) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: "`--debug-bundle-path` can only be used with `--once`.",
      });
    }

    if (cmdOptions.configure === undefined) {
      if (cmdOptions.team || cmdOptions.project)
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage:
            "`--team and --project can can only be used with `--configure`.",
        });
    }

    const localOptions: {
      ports?: { cloud: number; site: number };
      backendVersion?: string | undefined;
      forceUpgrade: boolean;
    } = { forceUpgrade: false };
    if (!cmdOptions.local) {
      if (
        cmdOptions.localCloudPort !== undefined ||
        cmdOptions.localSitePort !== undefined ||
        cmdOptions.localBackendVersion !== undefined ||
        cmdOptions.localForceUpgrade === true
      ) {
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage:
            "`--local-*` options can only be used with `--local`.",
        });
      }
    } else {
      if (cmdOptions.localCloudPort !== undefined) {
        if (cmdOptions.localSitePort === undefined) {
          return await ctx.crash({
            exitCode: 1,
            errorType: "fatal",
            printedMessage:
              "`--local-cloud-port` requires `--local-site-port` to be set.",
          });
        }
        localOptions["ports"] = {
          cloud: parseInt(cmdOptions.localCloudPort),
          site: parseInt(cmdOptions.localSitePort),
        };
      }
      localOptions["backendVersion"] = cmdOptions.localBackendVersion;
      localOptions["forceUpgrade"] = cmdOptions.localForceUpgrade;
    }

    if (!cmdOptions.url || !cmdOptions.adminKey) {
      if (!(await checkAuthorization(ctx, false))) {
        await performLogin(ctx, cmdOptions);
      }
    }

    const partitionId = cmdOptions.partitionId
      ? parseInt(cmdOptions.partitionId)
      : undefined;
    const configure =
      cmdOptions.configure === true ? "ask" : (cmdOptions.configure ?? null);
    const credentials = await deploymentCredentialsOrConfigure(
      ctx,
      configure,
      {
        ...cmdOptions,
        localOptions,
      },
      partitionId,
    );

    await usageStateWarning(ctx);

    if (cmdOptions.skipPush) {
      return;
    }

    const promises = [];
    if (cmdOptions.tailLogs) {
      promises.push(
        watchLogs(ctx, credentials.url, credentials.adminKey, "stderr"),
      );
    }
    promises.push(
      watchAndPush(
        ctx,
        {
          ...credentials,
          verbose: !!cmdOptions.verbose,
          dryRun: false,
          typecheck: cmdOptions.typecheck,
          typecheckComponents: !!cmdOptions.typecheckComponents,
          debug: false,
          debugBundlePath: cmdOptions.debugBundlePath,
          codegen: cmdOptions.codegen === "enable",
          liveComponentSources: !!cmdOptions.liveComponentSources,
        },
        cmdOptions,
      ),
    );
    await Promise.race(promises);
    await ctx.flushAndExit(0);
  });

export async function watchAndPush(
  outerCtx: OneoffCtx,
  options: PushOptions,
  cmdOptions: {
    run?: string;
    runComponent?: string;
    once: boolean;
    untilSuccess: boolean;
    traceEvents: boolean;
  },
) {
  const watch: { watcher: Watcher | undefined } = { watcher: undefined };
  let numFailures = 0;
  let ran = false;
  let pushed = false;
  let tableNameTriggeringRetry;
  let shouldRetryOnDeploymentEnvVarChange;

  while (true) {
    const start = performance.now();
    tableNameTriggeringRetry = null;
    shouldRetryOnDeploymentEnvVarChange = false;
    const ctx = new WatchContext(cmdOptions.traceEvents);
    showSpinner(ctx, "Preparing Convex functions...");
    try {
      await runPush(ctx, options);
      const end = performance.now();
      numFailures = 0;
      logFinishedStep(
        ctx,
        `${getCurrentTimeString()} Convex functions ready! (${formatDuration(
          end - start,
        )})`,
      );
      if (cmdOptions.run !== undefined && !ran) {
        await runFunctionInDev(
          ctx,
          options,
          cmdOptions.run,
          cmdOptions.runComponent,
        );
        ran = true;
      }
      pushed = true;
    } catch (e: any) {
      // Crash the app on unexpected errors.
      if (!(e instanceof Crash) || !e.errorType) {
        // eslint-disable-next-line no-restricted-syntax
        throw e;
      }
      if (e.errorType === "fatal") {
        break;
      }
      // Retry after an exponential backoff if we hit a transient error.
      if (e.errorType === "transient") {
        const delay = nextBackoff(numFailures);
        numFailures += 1;
        logWarning(
          ctx,
          chalk.yellow(
            `Failed due to network error, retrying in ${formatDuration(
              delay,
            )}...`,
          ),
        );
        await new Promise((resolve) => setTimeout(resolve, delay));
        continue;
      }

      // Fall through if we had a filesystem-based error.
      // TODO(sarah): Replace this with `logError`.
      // eslint-disable-next-line no-console
      console.assert(
        e.errorType === "invalid filesystem data" ||
          e.errorType === "invalid filesystem or env vars" ||
          e.errorType["invalid filesystem or db data"] !== undefined,
      );
      if (e.errorType === "invalid filesystem or env vars") {
        shouldRetryOnDeploymentEnvVarChange = true;
      } else if (
        e.errorType !== "invalid filesystem data" &&
        e.errorType["invalid filesystem or db data"] !== undefined
      ) {
        tableNameTriggeringRetry = e.errorType["invalid filesystem or db data"];
      }
      if (cmdOptions.once) {
        await outerCtx.flushAndExit(1, e.errorType);
      }
      // Make sure that we don't spin if this push failed
      // in any edge cases that didn't call `logFailure`
      // before throwing.
      stopSpinner(ctx);
    }
    if (cmdOptions.once) {
      return;
    }
    if (pushed && cmdOptions.untilSuccess) {
      return;
    }
    const fileSystemWatch = getFileSystemWatch(ctx, watch, cmdOptions);
    const tableWatch = getTableWatch(
      ctx,
      options,
      tableNameTriggeringRetry?.tableName ?? null,
      tableNameTriggeringRetry?.componentPath,
    );
    const envVarWatch = getDeplymentEnvVarWatch(
      ctx,
      options,
      shouldRetryOnDeploymentEnvVarChange,
    );
    await Promise.race([
      fileSystemWatch.watch(),
      tableWatch.watch(),
      envVarWatch.watch(),
    ]);
    fileSystemWatch.stop();
    void tableWatch.stop();
    void envVarWatch.stop();
  }
}

async function runFunctionInDev(
  ctx: WatchContext,
  credentials: {
    url: string;
    adminKey: string;
  },
  functionName: string,
  componentPath: string | undefined,
) {
  await runFunctionAndLog(ctx, {
    deploymentUrl: credentials.url,
    adminKey: credentials.adminKey,
    functionName,
    argsString: "{}",
    componentPath,
    callbacks: {
      onSuccess: () => {
        logFinishedStep(ctx, `Finished running function "${functionName}"`);
      },
    },
  });
}

function getTableWatch(
  ctx: WatchContext,
  credentials: {
    url: string;
    adminKey: string;
  },
  tableName: string | null,
  componentPath: string | undefined,
) {
  return getFunctionWatch(ctx, {
    deploymentUrl: credentials.url,
    adminKey: credentials.adminKey,
    parsedFunctionName: "_system/cli/queryTable",
    getArgs: () => (tableName !== null ? { tableName } : null),
    componentPath,
  });
}

function getDeplymentEnvVarWatch(
  ctx: WatchContext,
  credentials: {
    url: string;
    adminKey: string;
  },
  shouldRetryOnDeploymentEnvVarChange: boolean,
) {
  return getFunctionWatch(ctx, {
    deploymentUrl: credentials.url,
    adminKey: credentials.adminKey,
    parsedFunctionName: "_system/cli/queryEnvironmentVariables",
    getArgs: () => (shouldRetryOnDeploymentEnvVarChange ? {} : null),
    componentPath: undefined,
  });
}

function getFunctionWatch(
  ctx: WatchContext,
  args: {
    deploymentUrl: string;
    adminKey: string;
    parsedFunctionName: string;
    getArgs: () => Record<string, Value> | null;
    componentPath: string | undefined;
  },
) {
  const [stopPromise, stop] = waitUntilCalled();
  return {
    watch: async () => {
      const functionArgs = args.getArgs();
      if (functionArgs === null) {
        return waitForever();
      }
      let changes = 0;
      return subscribe(ctx, {
        deploymentUrl: args.deploymentUrl,
        adminKey: args.adminKey,
        parsedFunctionName: args.parsedFunctionName,
        parsedFunctionArgs: functionArgs,
        componentPath: args.componentPath,
        until: stopPromise,
        callbacks: {
          onChange: () => {
            changes++;
            // First bump is just the initial results reporting
            if (changes > 1) {
              stop();
            }
          },
        },
      });
    },
    stop: () => {
      stop();
    },
  };
}

function getFileSystemWatch(
  ctx: WatchContext,
  watch: { watcher: Watcher | undefined },
  cmdOptions: { traceEvents: boolean },
) {
  let hasStopped = false;
  return {
    watch: async () => {
      const observations = ctx.fs.finalize();
      if (observations === "invalidated") {
        logMessage(ctx, "Filesystem changed during push, retrying...");
        return;
      }
      // Initialize the watcher if we haven't done it already. Chokidar expects to have a
      // nonempty watch set at initialization, so we can't do it before running our first
      // push.
      if (!watch.watcher) {
        watch.watcher = new Watcher(observations);
        await showSpinnerIfSlow(
          ctx,
          "Preparing to watch files...",
          500,
          async () => {
            await watch.watcher!.ready();
          },
        );
        stopSpinner(ctx);
      }
      // Watch new directories if needed.
      watch.watcher.update(observations);

      // Process events until we find one that overlaps with our previous observations.
      let anyChanges = false;
      do {
        await watch.watcher.waitForEvent();
        if (hasStopped) {
          return;
        }
        for (const event of watch.watcher.drainEvents()) {
          if (cmdOptions.traceEvents) {
            logMessage(
              ctx,
              "Processing",
              event.name,
              path.relative("", event.absPath),
            );
          }
          const result = observations.overlaps(event);
          if (result.overlaps) {
            const relPath = path.relative("", event.absPath);
            if (cmdOptions.traceEvents) {
              logMessage(ctx, `${relPath} ${result.reason}, rebuilding...`);
            }
            anyChanges = true;
            break;
          }
        }
      } while (!anyChanges);

      // Wait for the filesystem to quiesce before starting a new push. It's okay to
      // drop filesystem events at this stage since we're already committed to doing
      // a push and resubscribing based on that push's observations.
      let deadline = performance.now() + quiescenceDelay;
      while (true) {
        const now = performance.now();
        if (now >= deadline) {
          break;
        }
        const remaining = deadline - now;
        if (cmdOptions.traceEvents) {
          logMessage(
            ctx,
            `Waiting for ${formatDuration(remaining)} to quiesce...`,
          );
        }
        const remainingWait = new Promise<"timeout">((resolve) =>
          setTimeout(() => resolve("timeout"), deadline - now),
        );
        const result = await Promise.race([
          remainingWait,
          watch.watcher.waitForEvent().then<"newEvents">(() => "newEvents"),
        ]);
        if (result === "newEvents") {
          for (const event of watch.watcher.drainEvents()) {
            const result = observations.overlaps(event);
            // Delay another `quiescenceDelay` since we had an overlapping event.
            if (result.overlaps) {
              if (cmdOptions.traceEvents) {
                logMessage(
                  ctx,
                  `Received an overlapping event at ${event.absPath}, delaying push.`,
                );
              }
              deadline = performance.now() + quiescenceDelay;
            }
          }
        } else {
          // Let the check above `break` from the loop if we're past our deadlne.
          if (result !== "timeout") {
            logError(
              ctx,
              "Assertion failed: Unexpected result from watcher: " + result,
            );
          }
        }
      }
    },
    stop: () => {
      hasStopped = true;
    },
  };
}

const initialBackoff = 500;
const maxBackoff = 16000;
const quiescenceDelay = 500;

export function nextBackoff(prevFailures: number): number {
  const baseBackoff = initialBackoff * Math.pow(2, prevFailures);
  const actualBackoff = Math.min(baseBackoff, maxBackoff);
  const jitter = actualBackoff * (Math.random() - 0.5);
  return actualBackoff + jitter;
}
