import * as Sentry from "@sentry/node";
import chalk from "chalk";
import ora, { Ora } from "ora";
import { Filesystem, nodeFs } from "./fs.js";
import { format } from "util";
import ProgressBar from "progress";
import { initializeBigBrainAuth } from "../cli/lib/deploymentSelection.js";
// How the error should be handled when running `npx convex dev`.
export type ErrorType =
  // The error was likely caused by the state of the developer's local
  // file system (e.g. `tsc` fails due to a syntax error). The `convex dev`
  // command will then print out the error and wait for the file to change before
  // retrying.
  | "invalid filesystem data"
  // The error was caused by either the local state (ie schema.ts content)
  // or the state of the db (ie documents not matching the new schema).
  // The `convex dev` command will wait for either file OR table data change
  // to retry (if a table name is specified as the value in this Object).
  | {
      "invalid filesystem or db data": {
        tableName: string;
        componentPath?: string;
      } | null;
    }
  // The error was caused by either the local state (ie schema.ts content)
  // or the state of the deployment environment variables.
  // The `convex dev` command will wait for either file OR env var change
  // before retrying.
  | "invalid filesystem or env vars"
  // The error was some transient issue (e.g. a network
  // error). This will then cause a retry after an exponential backoff.
  | "transient"
  // This error is truly permanent. Exit `npx convex dev` because the
  // developer will need to take a manual commandline action.
  | "fatal";

export type BigBrainAuth = {
  header: string;
} & (
  | {
      kind: "projectKey";
      projectKey: string;
    }
  | {
      kind: "previewDeployKey";
      previewDeployKey: string;
    }
  | {
      kind: "accessToken";
      accessToken: string;
    }
);

export interface Context {
  fs: Filesystem;
  deprecationMessagePrinted: boolean;
  spinner: Ora | undefined;
  // Reports to Sentry and either throws FatalError or exits the process.
  // Prints the `printedMessage` if provided
  crash(args: {
    exitCode: number;
    errorType: ErrorType;
    errForSentry?: any;
    printedMessage: string | null;
  }): Promise<never>;
  registerCleanup(fn: (exitCode: number, err?: any) => Promise<void>): string;
  removeCleanup(
    handle: string,
  ): (exitCode: number, err?: any) => Promise<void> | null;
  bigBrainAuth(): BigBrainAuth | null;
  /**
   * Prefer using `updateBigBrainAuthAfterLogin` in `deploymentSelection.ts` instead
   */
  _updateBigBrainAuth(auth: BigBrainAuth | null): void;
}

async function flushAndExit(exitCode: number, err?: any) {
  if (err) {
    Sentry.captureException(err);
  }
  await Sentry.close();
  return process.exit(exitCode);
}

export type OneoffCtx = Context & {
  // Generally `ctx.crash` is better to use since it handles printing a message
  // for the user, and then calls this.
  //
  // This function reports to Sentry + exits the process, but does not handle
  // printing a message for the user.
  flushAndExit: (exitCode: number, err?: any) => Promise<never>;
};

class OneoffContextImpl {
  private _cleanupFns: Record<
    string,
    (exitCode: number, err?: any) => Promise<void>
  > = {};
  public fs: Filesystem = nodeFs;
  public deprecationMessagePrinted: boolean = false;
  public spinner: Ora | undefined = undefined;
  private _bigBrainAuth: BigBrainAuth | null = null;

  crash = async (args: {
    exitCode: number;
    errorType?: ErrorType;
    errForSentry?: any;
    printedMessage: string | null;
  }) => {
    if (args.printedMessage !== null) {
      logFailure(this, args.printedMessage);
    }
    return await this.flushAndExit(args.exitCode, args.errForSentry);
  };
  flushAndExit = async (exitCode: number, err?: any) => {
    logVerbose(this, "Flushing and exiting, error:", err);
    if (err) {
      logVerbose(this, err.stack);
    }
    const cleanupFns = this._cleanupFns;
    // Clear the cleanup functions so that there's no risk of running them twice
    // if this somehow gets triggered twice.
    this._cleanupFns = {};
    const fns = Object.values(cleanupFns);
    logVerbose(this, `Running ${fns.length} cleanup functions`);
    for (const fn of fns) {
      await fn(exitCode, err);
    }
    logVerbose(this, "All cleanup functions ran");
    return flushAndExit(exitCode, err);
  };
  registerCleanup(fn: (exitCode: number, err?: any) => Promise<void>) {
    const handle = Math.random().toString(36).slice(2);
    this._cleanupFns[handle] = fn;
    return handle;
  }
  removeCleanup(handle: string) {
    const value = this._cleanupFns[handle];
    delete this._cleanupFns[handle];
    return value ?? null;
  }
  bigBrainAuth(): BigBrainAuth | null {
    return this._bigBrainAuth;
  }
  _updateBigBrainAuth(auth: BigBrainAuth | null): void {
    logVerbose(this, `Updating big brain auth to ${auth?.kind ?? "null"}`);
    this._bigBrainAuth = auth;
  }
}

export const oneoffContext: (args: {
  url?: string;
  adminKey?: string;
  envFile?: string;
}) => Promise<OneoffCtx> = async (args) => {
  const ctx = new OneoffContextImpl();
  await initializeBigBrainAuth(ctx, {
    url: args.url,
    adminKey: args.adminKey,
    envFile: args.envFile,
  });
  return ctx;
};
// console.error before it started being red by default in Node v20
function logToStderr(...args: unknown[]) {
  process.stderr.write(`${format(...args)}\n`);
}

// Handles clearing spinner so that it doesn't get messed up
export function logError(ctx: Context, message: string) {
  ctx.spinner?.clear();
  logToStderr(message);
}

// Handles clearing spinner so that it doesn't get messed up
export function logWarning(ctx: Context, ...logged: any) {
  ctx.spinner?.clear();
  logToStderr(...logged);
}

// Handles clearing spinner so that it doesn't get messed up
export function logMessage(ctx: Context, ...logged: any) {
  ctx.spinner?.clear();
  logToStderr(...logged);
}

// For the rare case writing output to stdout. Status and error messages
// (logMessage, logWarning, etc.) should be written to stderr.
export function logOutput(ctx: Context, ...logged: any) {
  ctx.spinner?.clear();
  // the one spot where we can console.log
  // eslint-disable-next-line no-console
  console.log(...logged);
}

export function logVerbose(ctx: Context, ...logged: any) {
  if (process.env.CONVEX_VERBOSE) {
    logMessage(ctx, `[verbose] ${new Date().toISOString()}`, ...logged);
  }
}

/**
 * Returns a ProgressBar instance, and also handles clearing the spinner if necessary.
 *
 * The caller is responsible for calling `progressBar.tick()` and terminating the `progressBar`
 * when it's done.
 */
export function startLogProgress(
  ctx: Context,
  format: string,
  progressBarOptions: ProgressBar.ProgressBarOptions,
): ProgressBar {
  ctx.spinner?.clear();
  return new ProgressBar(format, progressBarOptions);
}

// Start a spinner.
// To change its message use changeSpinner.
// To print warnings/errors while it's running use logError or logWarning.
// To stop it due to an error use logFailure.
// To stop it due to success use logFinishedStep.
export function showSpinner(ctx: Context, message: string) {
  ctx.spinner?.stop();
  ctx.spinner = ora({
    // Add newline to prevent clobbering when a message
    // we can't pipe through `logMessage` et al gets printed
    text: message + "\n",
    stream: process.stderr,
    // hideCursor: true doesn't work with `tsx`.
    // see https://github.com/tapjs/signal-exit/issues/49#issuecomment-1459408082
    // See CX-6822 for an issue to bring back cursor hiding, probably by upgrading libraries.
    hideCursor: process.env.CONVEX_RUNNING_LIVE_IN_MONOREPO ? false : true,
  }).start();
}

export function changeSpinner(ctx: Context, message: string) {
  if (ctx.spinner) {
    // Add newline to prevent clobbering
    ctx.spinner.text = message + "\n";
  } else {
    logToStderr(message);
  }
}

export function logFailure(ctx: Context, message: string) {
  if (ctx.spinner) {
    ctx.spinner.fail(message);
    ctx.spinner = undefined;
  } else {
    logToStderr(`${chalk.red(`✖`)} ${message}`);
  }
}

// Stops and removes spinner if one is active
export function logFinishedStep(ctx: Context, message: string) {
  if (ctx.spinner) {
    ctx.spinner.succeed(message);
    ctx.spinner = undefined;
  } else {
    logToStderr(`${chalk.green(`✔`)} ${message}`);
  }
}

export function stopSpinner(ctx: Context) {
  if (ctx.spinner) {
    ctx.spinner.stop();
    ctx.spinner = undefined;
  }
}

// Only shows the spinner if the async `fn` takes longer than `delayMs`
export async function showSpinnerIfSlow(
  ctx: Context,
  message: string,
  delayMs: number,
  fn: () => Promise<any>,
) {
  const timeout = setTimeout(() => {
    showSpinner(ctx, message);
  }, delayMs);
  await fn();
  clearTimeout(timeout);
}
