import ProgressBar from "progress";
import * as Sentry from "@sentry/node";
import chalk from "chalk";
import ora, { Ora } from "ora";
import { format } from "util";
import { Node } from "ts-morph";
import { relative } from "path";
import { generateCodeframe } from "./util/codeframe";

export interface Context {
  spinner: Ora | undefined;

  // Reports to Sentry and either throws FatalError or exits the process.
  // Prints the `printedMessage` if provided
  crash(args: {
    exitCode?: number;
    errForSentry?: any;
    printedMessage: string | null;
  }): Promise<never>;

  addWarning(args: { title: string; message: string; node: Node }): void;

  incrementChanges(file: string): void;

  printResults(isDryRun: boolean): void;
}

async function flushAndExit(exitCode: number, err?: any) {
  if (err) {
    Sentry.captureException(err);
  }
  await Sentry.close();
  return process.exit(exitCode);
}

class ContextImpl implements Context {
  public spinner: Ora | undefined = undefined;
  private warnings: { title: string; message: string; node: Node }[] = [];
  private changes: { [file: string]: number } = {};

  crash = async (args: {
    exitCode?: number;
    errForSentry?: any;
    printedMessage: string | null;
  }) => {
    if (args.printedMessage !== null) {
      logFailure(this, args.printedMessage);
    }
    return await this.flushAndExit(args.exitCode ?? 1, args.errForSentry);
  };

  flushAndExit = async (exitCode: number, err?: any) => {
    logVerbose(this, "Flushing and exiting, error:", err);
    if (err) {
      logVerbose(this, err.stack);
    }

    return flushAndExit(exitCode, err);
  };

  addWarning = (warning: { title: string; message: string; node: Node }) => {
    this.warnings.push(warning);
  };

  incrementChanges = (file: string) => {
    this.changes[file] = (this.changes[file] ?? 0) + 1;
  };

  printResults = (isDryRun: boolean) => {
    this.printWarnings();
    this.printSuccessStep(isDryRun);
  };

  private printSuccessStep(isDryRun: boolean) {
    const warningText =
      this.warnings.length > 0
        ? ` Emitted ${chalk.bold(this.warnings.length)} warnings.`
        : "";

    if (Object.keys(this.changes).length === 0) {
      logFinishedStep(this, "Nothing was changed." + warningText);
    } else {
      const changesCount = Object.values(this.changes).reduce(
        (acc, count) => acc + count,
        0,
      );
      const changedFilesCount = Object.keys(this.changes).length;
      logFinishedStep(
        this,
        `${isDryRun ? "Would have updated" : "Updated"} ${chalk.bold(changesCount)} ${
          changesCount === 1 ? "call site" : "call sites"
        } over ${chalk.bold(changedFilesCount)} ${
          changedFilesCount === 1 ? "file" : "files"
        }.` + warningText,
      );
    }
  }

  private printWarnings() {
    for (const warning of this.warnings) {
      const codeBlock = generateCodeframe(warning.node, warning.message);

      logWarning(
        this,
        chalk.bold(warning.title) +
          "\n" +
          chalk.gray(
            relative(
              process.cwd(),
              warning.node.getSourceFile().getFilePath(),
            ) +
              ":" +
              warning.node.getStartLineNumber() +
              ":" +
              warning.node.getStartLinePos(),
          ) +
          "\n" +
          codeBlock +
          "\n",
      );
    }
  }
}

export const createContext = () => {
  return new ContextImpl();
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
  logToStderr(chalk.yellow(`⚠`), ...logged);
}

// Handles clearing spinner so that it doesn't get messed up
export function logMessage(ctx: Context, ...logged: any) {
  ctx.spinner?.clear();
  logToStderr(...logged);
}

// For the rare case writing output to stdout. Status and error messages
// (logMesage, logWarning, etc.) should be written to stderr.
export function logOutput(ctx: Context, ...logged: any) {
  ctx.spinner?.clear();
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
