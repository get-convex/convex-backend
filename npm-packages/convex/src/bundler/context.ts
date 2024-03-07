import * as Sentry from "@sentry/node";
import chalk from "chalk";
import ora, { Ora } from "ora";
import { Filesystem, nodeFs } from "./fs.js";

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
      "invalid filesystem or db data": string | null;
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

export interface Context {
  fs: Filesystem;
  deprecationMessagePrinted: boolean;
  spinner: Ora | undefined;
  // Reports to Sentry and either throws FatalError or exits the process.
  // Does not print the error.
  crash(exitCode: number, errorType?: ErrorType, err?: any): Promise<never>;
}

export const oneoffContext: Context = {
  fs: nodeFs,
  deprecationMessagePrinted: false,
  spinner: undefined,
  async crash(exitCode: number, _errorType?: ErrorType, err?: any) {
    return await flushAndExit(exitCode, err);
  },
};

async function flushAndExit(exitCode: number, err?: any) {
  if (err) {
    Sentry.captureException(err);
  }
  await Sentry.close();
  // eslint-disable-next-line no-restricted-syntax
  return process.exit(exitCode);
}

// Handles clearing spinner so that it doesn't get messed up
export function logError(ctx: Context, message: string) {
  ctx.spinner?.clear();
  console.error(message);
}

// Handles clearing spinner so that it doesn't get messed up
export function logWarning(ctx: Context, message: string) {
  ctx.spinner?.clear();
  console.error(message);
}

// Handles clearing spinner so that it doesn't get messed up
export function logMessage(ctx: Context, ...logged: any) {
  ctx.spinner?.clear();
  console.error(...logged);
}

// For the rare case writing output to stdout. Status and error messages
// (logMesage, logWarning, etc.) should be written to stderr.
export function logOutput(ctx: Context, ...logged: any) {
  ctx.spinner?.clear();
  console.log(...logged);
}

// Start a spinner.
// To change its message use changeSpinner.
// To print warnings/erros while it's running use logError or logWarning.
// To stop it due to an error use logFailure.
// To stop it due to success use logFinishedStep.
export function showSpinner(ctx: Context, message: string) {
  ctx.spinner?.stop();
  ctx.spinner = ora({
    // Add newline to prevent clobbering when a message
    // we can't pipe through `logMessage` et al gets printed
    text: message + "\n",
    stream: process.stderr,
  }).start();
}

export function changeSpinner(ctx: Context, message: string) {
  if (ctx.spinner) {
    // Add newline to prevent clobbering
    ctx.spinner.text = message + "\n";
  } else {
    console.error(message);
  }
}

export function logFailure(ctx: Context, message: string) {
  if (ctx.spinner) {
    ctx.spinner.fail(message);
    ctx.spinner = undefined;
  } else {
    console.error(`${chalk.red(`✖`)} ${message}`);
  }
}

// Stops and removes spinner if one is active
export function logFinishedStep(ctx: Context, message: string) {
  if (ctx.spinner) {
    ctx.spinner.succeed(message);
    ctx.spinner = undefined;
  } else {
    console.error(`${chalk.green(`✔`)} ${message}`);
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
