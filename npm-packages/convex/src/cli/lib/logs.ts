import {
  Context,
  logMessage,
  logOutput,
  logWarning,
} from "../../bundler/context.js";
import { nextBackoff } from "../dev.js";
import chalk from "chalk";
import { deploymentFetch } from "./utils/utils.js";

const MAX_UDF_STREAM_FAILURE_COUNT = 5;

type LogDestination = "stdout" | "stderr";

export async function watchLogs(
  ctx: Context,
  url: string,
  adminKey: string,
  dest: LogDestination,
  options?: {
    success: boolean;
    history?: number | boolean;
  },
) {
  let numFailures = 0;
  let isFirst = true;
  let cursorMs = 0;

  for (;;) {
    try {
      const { entries, newCursor } = await pollUdfLog(cursorMs, url, adminKey);
      cursorMs = newCursor;
      numFailures = 0;
      // The first execution, we just want to fetch the current head cursor so we don't send stale
      // logs to the client.
      if (isFirst) {
        isFirst = false;
        if (
          options?.history === true ||
          (typeof options?.history === "number" && options?.history > 0)
        ) {
          const entriesSlice =
            options?.history === true
              ? entries
              : entries.slice(entries.length - options?.history);
          processLogs(ctx, entriesSlice, dest, options?.success);
        }
      } else {
        processLogs(ctx, entries, dest, options?.success === true);
      }
    } catch (e) {
      numFailures += 1;
    }
    // Handle backoff
    if (numFailures > 0) {
      const backoff = nextBackoff(numFailures);

      // If we exceed a threshold number of failures, warn the user and display backoff.
      if (numFailures > MAX_UDF_STREAM_FAILURE_COUNT) {
        logWarning(
          ctx,
          `Convex [WARN] Failed to fetch logs. Waiting ${backoff}ms before next retry.`,
        );
      }
      await new Promise((resolve) => {
        setTimeout(() => resolve(null), backoff);
      });
    }
  }
}

type UdfType = "Query" | "Mutation" | "Action" | "HttpAction";

type StructuredLogLine = {
  messages: string[];
  level: "LOG" | "DEBUG" | "INFO" | "WARN" | "ERROR";
  timestamp: number;
  isTruncated: boolean;
};
type LogLine = string | StructuredLogLine;

type UdfExecutionResponse = {
  identifier: string;
  udfType: UdfType;
  logLines: LogLine[];
  // Unix timestamp (in seconds)
  timestamp: number;
  // UDF execution duration (in seconds)
  executionTime: number;
  error: string | null;
  kind: "Completion" | "Progress";
};

async function pollUdfLog(
  cursor: number,
  url: string,
  adminKey: string,
): Promise<{ entries: UdfExecutionResponse[]; newCursor: number }> {
  const fetch = deploymentFetch(url, adminKey);
  const response = await fetch(`/api/stream_function_logs?cursor=${cursor}`, {
    method: "GET",
  });
  return await response.json();
}

const prefixForSource = (udfType: UdfType): string => {
  return udfType.charAt(0);
};

function processLogs(
  ctx: Context,
  rawLogs: UdfExecutionResponse[],
  dest: LogDestination,
  shouldShowSuccessLogs: boolean,
) {
  for (let i = 0; i < rawLogs.length; i++) {
    const log = rawLogs[i];
    if (log.logLines) {
      const id = log.identifier;
      const udfType = log.udfType;
      const timestampMs = log.timestamp * 1000;
      const executionTimeMs = log.executionTime * 1000;

      for (let j = 0; j < log.logLines.length; j++) {
        logToTerminal(
          ctx,
          "info",
          timestampMs,
          udfType,
          id,
          log.logLines[j],
          dest,
        );
      }

      if (log.error) {
        logToTerminal(ctx, "error", timestampMs, udfType, id, log.error!, dest);
      } else if (log.kind === "Completion" && shouldShowSuccessLogs) {
        logFunctionExecution(
          ctx,
          timestampMs,
          log.udfType,
          id,
          executionTimeMs,
          dest,
        );
      }
    }
  }
}

function logFunctionExecution(
  ctx: Context,
  timestampMs: number,
  udfType: UdfType,
  udfPath: string,
  executionTimeMs: number,
  dest: LogDestination,
) {
  logToDestination(
    ctx,
    dest,
    chalk.green(
      `${prefixLog(
        timestampMs,
        udfType,
        udfPath,
      )} Function executed in ${Math.ceil(executionTimeMs)} ms`,
    ),
  );
}

function logToTerminal(
  ctx: Context,
  type: "info" | "error",
  timestampMs: number,
  udfType: UdfType,
  udfPath: string,
  message: LogLine,
  dest: LogDestination,
) {
  const prefix = prefixForSource(udfType);
  if (typeof message === "string") {
    if (type === "info") {
      const match = message.match(/^\[.*?\] /);
      if (match === null) {
        logToDestination(
          ctx,
          dest,
          chalk.red(
            `[CONVEX ${prefix}(${udfPath})] Could not parse console.log`,
          ),
        );
        return;
      }
      const level = message.slice(1, match[0].length - 2);
      const args = message.slice(match[0].length);

      logToDestination(
        ctx,
        dest,
        chalk.cyan(`${prefixLog(timestampMs, udfType, udfPath)} [${level}]`),
        args,
      );
    } else {
      logToDestination(
        ctx,
        dest,
        chalk.red(`${prefixLog(timestampMs, udfType, udfPath)} ${message}`),
      );
    }
  } else {
    const level = message.level;
    const formattedMessage = `${message.messages.join(" ")}${message.isTruncated ? " (truncated due to length)" : ""}`;
    logToDestination(
      ctx,
      dest,
      chalk.cyan(
        // timestamp is in ms since epoch
        `${prefixLog(message.timestamp, udfType, udfPath)} [${level}]`,
      ),
      formattedMessage,
    );
  }
}

function logToDestination(ctx: Context, dest: LogDestination, ...logged: any) {
  switch (dest) {
    case "stdout":
      logOutput(ctx, ...logged);
      break;
    case "stderr":
      logMessage(ctx, ...logged);
      break;
  }
}

function prefixLog(timestampMs: number, udfType: UdfType, udfPath: string) {
  const prefix = prefixForSource(udfType);
  const localizedTimestamp = new Date(timestampMs).toLocaleString();

  return `${localizedTimestamp} [CONVEX ${prefix}(${udfPath})]`;
}
