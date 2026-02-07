import { UdfLog } from "@common/lib/useLogs";
import { DeploymentAuditLogEvent } from "@common/lib/useDeploymentAuditLog";

export type InterleavedLog =
  | {
      kind: "ExecutionLog";
      executionLog: UdfLog;
    }
  | {
      kind: "DeploymentEvent";
      deploymentEvent: DeploymentAuditLogEvent;
    }
  | {
      kind: "ClearedLogs";
      timestamp: number;
    }
  | {
      kind: "AggregatedLog";
      logs: InterleavedLog[];
      count: number;
    };

// Helper to get timestamp from InterleavedLog
export function getTimestamp(log: InterleavedLog): number {
  if (!log) {
    return 0;
  }
  switch (log.kind) {
    case "ExecutionLog":
      return log.executionLog.timestamp;
    case "DeploymentEvent":
      return log.deploymentEvent._creationTime;
    case "ClearedLogs":
      return log.timestamp;
    case "AggregatedLog":
      return getTimestamp(log.logs[0]);
    default:
      log satisfies never;
      return 0;
  }
}

/**
 * Get a unique key for an InterleavedLog that can be used for comparison.
 * Uses kind, timestamp, and id (if available) to ensure uniqueness.
 */
export function getLogKey(log: InterleavedLog): string {
  if (!log) {
    return "";
  }
  const timestamp = getTimestamp(log);
  if (log.kind === "ExecutionLog") {
    return `${log.kind}-${timestamp}-${log.executionLog.id}`;
  }
  if (log.kind === "DeploymentEvent") {
    return `${log.kind}-${timestamp}-${log.deploymentEvent._id}`;
  }
  if (log.kind === "AggregatedLog") {
    return `${log.kind}-${timestamp}-${getLogKey(log.logs[0])}`;
  }
  return `${log.kind}-${timestamp}`;
}

/**
 * Given two arrays of logs sorted from least recent to most recent, interleave
 * them based on time.
 * @param executionLogs
 * @param deploymentAuditLogEvents
 * @returns
 */

export function interleaveLogs(
  executionLogs: UdfLog[],
  deploymentAuditLogEvents: DeploymentAuditLogEvent[],
  clearedLogs: number[],
  shouldAggregateLogs: boolean = true,
): InterleavedLog[] {
  const result: InterleavedLog[] = [];

  const logIterator = executionLogs[Symbol.iterator]();
  const deploymentEventIterator = deploymentAuditLogEvents[Symbol.iterator]();
  const latestCleared = clearedLogs.at(-1);
  if (latestCleared !== undefined) {
    result.push({
      kind: "ClearedLogs",
      timestamp: latestCleared,
    });
  }

  let udfLog: UdfLog | undefined = logIterator.next().value;
  let deploymentEvent: DeploymentAuditLogEvent | undefined =
    deploymentEventIterator.next().value;

  while (udfLog !== undefined || deploymentEvent !== undefined) {
    if (
      udfLog &&
      (deploymentEvent === undefined ||
        udfLog.timestamp < deploymentEvent._creationTime)
    ) {
      if (latestCleared === undefined || udfLog.timestamp > latestCleared) {
        result.push({ kind: "ExecutionLog", executionLog: udfLog });
      }
      udfLog = logIterator.next().value;
    } else if (deploymentEvent) {
      if (
        latestCleared === undefined ||
        deploymentEvent._creationTime > latestCleared
      ) {
        result.push({
          kind: "DeploymentEvent",
          deploymentEvent,
        });
      }
      deploymentEvent = deploymentEventIterator.next().value;
    }
  }
  return shouldAggregateLogs ? aggregateInterleavedLogs(result) : result;
}

function aggregateInterleavedLogs(logs: InterleavedLog[]): InterleavedLog[] {
  const aggregated: InterleavedLog[] = [];
  for (const log of logs) {
    const last = aggregated[aggregated.length - 1];
    if (last && shouldAggregate(last, log)) {
      if (last.kind === "AggregatedLog") {
        last.logs.push(log);
        last.count++;
      } else {
        const firstLog = aggregated.pop()!;
        aggregated.push({
          kind: "AggregatedLog",
          logs: [firstLog, log],
          count: 2,
        });
      }
    } else {
      aggregated.push(log);
    }
  }
  return aggregated;
}

function shouldAggregate(a: InterleavedLog, b: InterleavedLog): boolean {
  const getComparisonKey = (log: InterleavedLog): string | null => {
    const item = log.kind === "AggregatedLog" ? log.logs[0] : log;
    if (item.kind !== "ExecutionLog" || item.executionLog.kind !== "log") {
      return null;
    }
    const { executionLog } = item;
    return `${executionLog.call}-${executionLog.output.level}-${executionLog.output.messages.join("|")}`;
  };

  const keyA = getComparisonKey(a);
  const keyB = getComparisonKey(b);

  return keyA !== null && keyA === keyB;
}
