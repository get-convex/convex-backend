import { Id } from "system-udfs/convex/_generated/dataModel";
import { UdfLog, DeploymentAuditLogEvent } from "dashboard-common";
import { interleaveLogs } from "./interleaveLogs";

function createUdfExecutionLog(creationTime: number): UdfLog {
  return {
    id: "1",
    kind: "log",
    executionId: `${creationTime}`,
    requestId: "myrequestid",
    localizedTimestamp: new Date(creationTime).toLocaleString(),
    timestamp: creationTime,
    udfType: "Mutation",
    call: "mutateData",
    output: { level: "DEBUG", messages: ["Log!"], isTruncated: false },
  };
}

function createDeploymentAuditLogEvent(
  creationTime: number,
): DeploymentAuditLogEvent {
  return {
    _id: "" as Id<"_deployment_audit_log">,
    _creationTime: creationTime,
    action: "push_config",
    metadata: {
      auth: {
        added: [],
        removed: [],
      },
      server_version: null,
      modules: {
        added: [],
        removed: [],
      },
    },
    memberName: "",
    member_id: BigInt(123),
  };
}

describe("interleaveLogs", () => {
  it("should work with no events", () => {
    expect(interleaveLogs([], [], [])).toEqual([]);
  });

  it("should work with no deployment logs", () => {
    const executionLogs = [
      createUdfExecutionLog(Date.parse("12/19/2022, 10:00:00 AM")),
      createUdfExecutionLog(Date.parse("12/19/2022, 10:01:00 AM")),
      createUdfExecutionLog(Date.parse("12/19/2022, 10:02:00 AM")),
    ];
    expect(interleaveLogs(executionLogs, [], [])).toEqual(
      executionLogs.map((executionLog) => ({
        kind: "ExecutionLog",
        executionLog,
      })),
    );
  });

  it("should work with no execution logs", () => {
    const deploymentLogs = [
      createDeploymentAuditLogEvent(Date.parse("12/19/2022, 10:00:00 AM")),
      createDeploymentAuditLogEvent(Date.parse("12/19/2022, 10:01:00 AM")),
      createDeploymentAuditLogEvent(Date.parse("12/19/2022, 10:02:00 AM")),
    ];
    expect(interleaveLogs([], deploymentLogs, [])).toEqual([
      {
        kind: "DeploymentEvent",
        deploymentEvent: deploymentLogs[0],
      },
      {
        kind: "DeploymentEvent",
        deploymentEvent: deploymentLogs[1],
      },
      {
        kind: "DeploymentEvent",
        deploymentEvent: deploymentLogs[2],
      },
    ]);
  });

  it("should interleave by time", () => {
    const executionLogA = createUdfExecutionLog(
      Date.parse("12/19/2022, 10:00:00 AM"),
    );
    const deploymentEventA = createDeploymentAuditLogEvent(
      Date.parse("12/19/2022, 10:01:00 AM"),
    );
    const executionLogB = createUdfExecutionLog(
      Date.parse("12/19/2022, 10:02:00 AM"),
    );
    const deploymentEventB = createDeploymentAuditLogEvent(
      Date.parse("12/19/2022, 10:03:00 AM"),
    );

    expect(
      interleaveLogs(
        [executionLogA, executionLogB],
        [deploymentEventA, deploymentEventB],
        [],
      ),
    ).toEqual([
      { kind: "ExecutionLog", executionLog: executionLogA },
      {
        kind: "DeploymentEvent",
        deploymentEvent: deploymentEventA,
      },
      { kind: "ExecutionLog", executionLog: executionLogB },
      {
        kind: "DeploymentEvent",
        deploymentEvent: deploymentEventB,
      },
    ]);
  });

  it("should group adjacent deployment events by time", () => {
    const executionLogA = createUdfExecutionLog(
      Date.parse("12/19/2022, 10:00:00 AM"),
    );
    const deploymentEventA = createDeploymentAuditLogEvent(
      Date.parse("12/19/2022, 10:01:00 AM"),
    );
    const deploymentEventB = createDeploymentAuditLogEvent(
      Date.parse("12/19/2022, 10:01:30 AM"),
    );
    const executionLogB = createUdfExecutionLog(
      Date.parse("12/19/2022, 10:02:00 AM"),
    );
    const deploymentEventC = createDeploymentAuditLogEvent(
      Date.parse("12/19/2022, 10:03:00 AM"),
    );

    expect(
      interleaveLogs(
        [executionLogA, executionLogB],
        [deploymentEventA, deploymentEventB, deploymentEventC],
        [],
      ),
    ).toEqual([
      { kind: "ExecutionLog", executionLog: executionLogA },
      {
        kind: "DeploymentEvent",
        deploymentEvent: deploymentEventA,
      },
      {
        kind: "DeploymentEvent",
        deploymentEvent: deploymentEventB,
      },
      { kind: "ExecutionLog", executionLog: executionLogB },
      {
        kind: "DeploymentEvent",
        deploymentEvent: deploymentEventC,
      },
    ]);
  });
});
