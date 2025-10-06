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
    };

// Helper to get timestamp from InterleavedLog
export function getTimestamp(log: InterleavedLog): number {
  switch (log.kind) {
    case "ExecutionLog":
      return log.executionLog.timestamp;
    case "DeploymentEvent":
      return log.deploymentEvent._creationTime;
    case "ClearedLogs":
      return log.timestamp;
    default:
      log satisfies never;
      return 0;
  }
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
  return result;
}
