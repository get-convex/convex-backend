import { UdfLog } from "@common/lib/useLogs";
import { InterleavedLog } from "@common/features/logs/lib/interleaveLogs";
import { messagesToString } from "@common/elements/LogOutput";
import {
  displayName,
  functionIdentifierFromValue,
} from "@common/lib/functions/generateFileTree";
import { msFormat } from "@common/lib/format";

/**
 * Formats a UdfLog into a clean, standardized string for copying or display.
 * Example: [2026-01-31 10:25:16.117] [INFO] my-function: Log message here
 */
export function formatUdfLogToString(log: UdfLog): string {
  const date = new Date(log.timestamp);
  const isoString = date.toISOString();
  // Format as: 2026-01-31 10:25:16.117
  const formattedTimestamp = isoString.replace("T", " ").replace("Z", "");

  let functionName: string;
  if (log.kind === "log" && log.output.subfunction) {
    const { identifier, componentPath } = functionIdentifierFromValue(
      log.output.subfunction,
    );
    functionName = displayName(identifier, componentPath);
  } else {
    const { identifier, componentPath } = functionIdentifierFromValue(log.call);
    functionName = displayName(identifier, componentPath);
  }

  const udfType = log.udfType.charAt(0).toUpperCase();

  let content = "";
  let level = "";
  let statusCode = "";

  if (log.kind === "log") {
    level = log.output.level ? `[${log.output.level}] ` : "";
    content = messagesToString(log.output);
  } else {
    level = "[OUTCOME] ";
    statusCode = log.outcome.statusCode ? `${log.outcome.statusCode} ` : "";
    content = log.error
      ? `${log.outcome.status} - ${log.error}`
      : log.outcome.status;
  }

  const executionTime =
    log.kind === "outcome" &&
    log.executionTimeMs !== null &&
    log.executionTimeMs > 0
      ? ` (${msFormat(log.executionTimeMs)})`
      : "";

  return `[${formattedTimestamp}] [${udfType}] ${level}${statusCode}${functionName}${executionTime}: ${content}`;
}

/**
 * Formats an InterleavedLog (which could be an Execution log or a Deployment event) into a string.
 */
export function formatInterleavedLogToString(log: InterleavedLog): string {
  switch (log.kind) {
    case "ExecutionLog":
      return formatUdfLogToString(log.executionLog);
    case "DeploymentEvent": {
      const ts = new Date(log.deploymentEvent._creationTime).toISOString();
      return `[${ts}] [SYSTEM] ${log.deploymentEvent.action}: ${JSON.stringify(log.deploymentEvent.metadata)}`;
    }
    case "ClearedLogs":
      return "--- Logs Cleared ---";
    default:
      return "";
  }
}
