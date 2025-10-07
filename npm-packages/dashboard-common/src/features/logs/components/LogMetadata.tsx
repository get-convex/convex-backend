import {
  ChevronDownIcon,
  ChevronUpIcon,
  PieChartIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import { useMemo } from "react";
import { UdfLog, UdfLogOutcome } from "@common/lib/useLogs";
import { Tooltip } from "@ui/Tooltip";
import { msFormat, formatBytes } from "@common/lib/format";
import { UsageStats } from "system-udfs/convex/_system/frontend/common";
import { FunctionNameOption } from "@common/elements/FunctionNameOption";
import { Disclosure } from "@headlessui/react";
import { Spinner } from "@ui/Spinner";

type RequestUsageStats = UsageStats & {
  actionsRuntimeMs: number;
  actionComputeMbMs: number;
  returnBytes?: number;
};

type OutcomeNode = {
  inProgress: boolean;
  functionName?: string;
  caller?: string;
  environment?: string;
  identityType?: string;
  executionTime?: number;
  localizedTimestamp?: string;
  endTime?: number;
  udfType?: string;
  cachedResult?: boolean;
};

export function LogMetadata({
  requestId,
  logs,
  executionId,
}: {
  requestId: string;
  logs: UdfLog[];
  executionId?: string;
}) {
  const isExecutionView = !!executionId;

  const filteredLogs = useMemo(() => {
    if (!isExecutionView) {
      return logs.filter((log) => log.requestId === requestId);
    }
    return logs.filter((log) => log.executionId === executionId);
  }, [logs, isExecutionView, executionId, requestId]);

  const requestOutcomeNode = useMemo((): OutcomeNode | null => {
    const outcomeLog =
      logs.find(
        (log): log is UdfLog & UdfLogOutcome =>
          log.kind === "outcome" &&
          log.requestId === requestId &&
          !("parentExecutionId" in log && log.parentExecutionId),
      ) ||
      logs.find(
        (log): log is UdfLog =>
          log.requestId === requestId &&
          !("parentExecutionId" in log && log.parentExecutionId),
      );

    return outcomeLog
      ? {
          inProgress: outcomeLog.kind !== "outcome",
          functionName: outcomeLog.call,
          caller: outcomeLog.kind === "outcome" ? outcomeLog.caller : undefined,
          environment:
            outcomeLog.kind === "outcome" ? outcomeLog.environment : undefined,
          identityType:
            outcomeLog.kind === "outcome" ? outcomeLog.identityType : undefined,
          executionTime:
            outcomeLog.kind === "outcome"
              ? (outcomeLog.executionTimeMs ?? undefined)
              : undefined,
          localizedTimestamp: outcomeLog.localizedTimestamp,
          endTime:
            "executionTimestamp" in outcomeLog
              ? outcomeLog.executionTimestamp
              : undefined,
          udfType: outcomeLog.udfType,
          cachedResult: outcomeLog.cachedResult,
        }
      : null;
  }, [logs, requestId]);

  const executionOutcomeNode = useMemo((): OutcomeNode | null => {
    if (!executionId) return null;

    const executionLogs = logs.filter((log) => log.executionId === executionId);

    // Find the outcome log for complete information
    const outcomeLog = executionLogs.find(
      (log): log is UdfLog & UdfLogOutcome => log.kind === "outcome",
    );

    // Get function name and type from any log with this executionId
    const anyLog = executionLogs[0];

    return {
      inProgress: !outcomeLog,
      functionName: outcomeLog?.call ?? anyLog?.call ?? undefined,
      caller: outcomeLog?.caller,
      environment: outcomeLog?.environment,
      identityType: outcomeLog?.identityType,
      executionTime: outcomeLog?.executionTimeMs ?? undefined,
      localizedTimestamp:
        outcomeLog?.localizedTimestamp ??
        anyLog?.localizedTimestamp ??
        undefined,
      endTime: outcomeLog?.executionTimestamp ?? undefined,
      udfType: outcomeLog?.udfType ?? anyLog?.udfType ?? undefined,
      cachedResult:
        outcomeLog?.cachedResult ?? anyLog?.cachedResult ?? undefined,
    };
  }, [logs, executionId]);

  const usageStats = useMemo(() => {
    const totals: RequestUsageStats = {
      actionMemoryUsedMb: 0,
      databaseReadBytes: 0,
      databaseReadDocuments: 0,
      databaseWriteBytes: 0,
      storageReadBytes: 0,
      storageWriteBytes: 0,
      vectorIndexReadBytes: 0,
      vectorIndexWriteBytes: 0,
      actionsRuntimeMs: 0,
      actionComputeMbMs: 0,
    };

    return filteredLogs.reduce((accumulated, log) => {
      const ret = accumulated;
      if ("usageStats" in log && log.usageStats) {
        for (const [key, value] of Object.entries(log.usageStats) as Array<
          [keyof UsageStats, number | null | undefined]
        >) {
          ret[key] += value ?? 0;
        }
      }
      if ("returnBytes" in log && log.returnBytes) {
        ret.returnBytes = (ret.returnBytes ?? 0) + log.returnBytes;
      }
      if (
        log.kind === "outcome" &&
        (log.udfType === "Action" || log.udfType === "HttpAction")
      ) {
        const durationMs = log.executionTimeMs ?? 0;
        ret.actionsRuntimeMs += durationMs;
        const memoryMb = (log.usageStats?.actionMemoryUsedMb ?? 0) as number;
        ret.actionComputeMbMs += durationMs * memoryMb;
      }
      return ret;
    }, totals);
  }, [filteredLogs]);

  const isInProgress = executionId
    ? !executionOutcomeNode || executionOutcomeNode.inProgress
    : !requestOutcomeNode || requestOutcomeNode.inProgress;

  return (
    <div className="animate-fadeInFromLoading p-2 text-xs">
      {isExecutionView ? (
        <ExecutionInfoList
          outcomeNode={executionOutcomeNode}
          executionId={executionId}
        />
      ) : (
        <RequestInfoList
          outcomeNode={requestOutcomeNode}
          requestId={requestId}
        />
      )}
      <ResourcesUsed
        usageStats={usageStats}
        filteredLogs={filteredLogs}
        isExecutionView={isExecutionView}
        isInProgress={isInProgress}
      />
    </div>
  );
}

function ResourcesUsed({
  usageStats,
  filteredLogs,
  isExecutionView,
  isInProgress,
}: {
  usageStats: RequestUsageStats;
  filteredLogs: UdfLog[];
  isExecutionView: boolean;
  isInProgress: boolean;
}) {
  return (
    <div className="mt-2">
      <Disclosure defaultOpen>
        {({ open }) => (
          <>
            <div className="flex items-center justify-between">
              <Disclosure.Button className="flex items-center gap-1 text-xs">
                <PieChartIcon className="size-3 text-content-secondary" />
                <h6 className="font-semibold text-content-secondary">
                  Resources Used
                </h6>
                {open ? (
                  <ChevronUpIcon className="size-3" />
                ) : (
                  <ChevronDownIcon className="size-3" />
                )}
              </Disclosure.Button>
            </div>

            <Disclosure.Panel className="mt-2 animate-fadeInFromLoading">
              {isInProgress && <Running />}

              {isInProgress && isExecutionView ? (
                <span className="mt-2 text-content-tertiary">
                  Resource usage will appear here once the function call
                  completes.
                </span>
              ) : (
                <ul className="divide-y text-xs">
                  <li className="grid min-w-fit grid-cols-2 items-center gap-2 py-1.5">
                    <span className="text-content-secondary">
                      Action Compute
                    </span>
                    <span className="min-w-0 text-content-primary">
                      <strong>
                        {Number(
                          usageStats.actionComputeMbMs / (1024 * 3_600_000),
                        ).toFixed(7)}{" "}
                        GB-hr
                      </strong>{" "}
                      ({usageStats.actionMemoryUsedMb ?? 0} MB for{" "}
                      {Number(usageStats.actionsRuntimeMs / 1000).toFixed(2)}s)
                    </span>
                  </li>
                  <li className="grid min-w-fit grid-cols-2 items-center gap-2 py-1.5">
                    <span className="text-content-secondary">DB Bandwidth</span>
                    <span className="min-w-0 text-content-primary">
                      Accessed{" "}
                      <strong>
                        {usageStats.databaseReadDocuments.toLocaleString()}{" "}
                        {usageStats.databaseReadDocuments === 1
                          ? "document"
                          : "documents"}
                      </strong>
                      ,{" "}
                      <strong>
                        {formatBytes(usageStats.databaseReadBytes)}
                      </strong>{" "}
                      read,{" "}
                      <strong>
                        {formatBytes(usageStats.databaseWriteBytes)}
                      </strong>{" "}
                      written
                    </span>
                  </li>
                  <li className="grid min-w-fit grid-cols-2 items-center gap-2 py-1.5">
                    <span className="text-content-secondary">
                      File Bandwidth
                    </span>
                    <span className="min-w-0 text-content-primary">
                      <strong>
                        {formatBytes(usageStats.storageReadBytes)}
                      </strong>{" "}
                      read,{" "}
                      <strong>
                        {formatBytes(usageStats.storageWriteBytes)}
                      </strong>{" "}
                      written
                    </span>
                  </li>
                  <li className="grid min-w-fit grid-cols-2 items-center gap-2 py-1.5">
                    <span className="text-content-secondary">
                      Vector Bandwidth
                    </span>
                    <span className="min-w-0 text-content-primary">
                      <strong>
                        {formatBytes(usageStats.vectorIndexReadBytes)}
                      </strong>{" "}
                      read,{" "}
                      <strong>
                        {formatBytes(usageStats.vectorIndexWriteBytes)}
                      </strong>{" "}
                      written
                    </span>
                  </li>
                  {usageStats.returnBytes && (
                    <li className="grid min-w-fit grid-cols-2 items-center gap-2 py-1.5">
                      <span className="flex items-center gap-1 text-content-secondary">
                        Return Size
                        <Tooltip tip="Bandwidth from sending the return value of a function call to the user does not incur costs.">
                          <QuestionMarkCircledIcon />
                        </Tooltip>
                      </span>
                      <span className="min-w-0 text-content-primary">
                        <strong>{formatBytes(usageStats.returnBytes)}</strong>{" "}
                        returned
                      </span>
                    </li>
                  )}
                  {filteredLogs.filter((log) => log.kind === "outcome").length >
                    1 && (
                    <li className="py-2 text-content-secondary">
                      Total resources used across{" "}
                      {filteredLogs.filter((l) => l.kind === "outcome").length}{" "}
                      executions
                      {isExecutionView
                        ? " in this execution"
                        : " in this request"}
                      .
                    </li>
                  )}
                </ul>
              )}
            </Disclosure.Panel>
          </>
        )}
      </Disclosure>
    </div>
  );
}

function FunctionEnvironment({ environment }: { environment?: string }) {
  switch (environment) {
    case "isolate":
      return (
        <div className="flex items-center gap-1">
          Convex
          <Tooltip tip="This function was executed in Convex's isolated environment.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      );
    case "node":
      return (
        <div className="flex items-center gap-1">
          Node
          <Tooltip tip="This function was executed in Convex's Node.js environment.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      );
    default:
      return <Running />;
  }
}

function FunctionIdentity({
  identity,
  caller,
}: {
  identity?: string;
  caller?: string;
}) {
  switch (identity) {
    case "instance_admin":
      return (
        <div className="flex items-center gap-1">
          Admin
          <Tooltip tip="This request was initiated by a Convex Developer with access to this deployment.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      );
    case "user":
      return (
        <div className="flex items-center gap-1">
          User
          <Tooltip tip="This request was initiated by a user.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      );
    case "member_acting_user":
    case "team_acting_user":
      return (
        <div className="flex items-center gap-1">
          Admin (Acting as user)
          <Tooltip tip="This request was initiated by a Convex Developer with access to this deployment while impersonating a user.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      );
    case "system":
      return (
        <div className="flex items-center gap-1">
          System
          <Tooltip tip="This request was initiatedby the Convex system.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      );
    case "unknown":
      return caller === "Scheduler" || caller === "Cron" ? (
        <div className="flex items-center gap-1">
          System
          <Tooltip tip="This function was executed by the Convex system.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      ) : (
        <div className="flex items-center gap-1">
          Unknown
          <Tooltip tip="This identity for this function call is unknown.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      );
    default:
      return <Running />;
  }
}

function FunctionCaller({ caller }: { caller?: string }) {
  switch (caller) {
    case "Tester":
      return (
        <div className="flex items-center gap-1">
          Function Runner
          <Tooltip tip="This function was executed through the Convex Dashboard or CLI.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      );
    case "HttpApi":
      return (
        <div className="flex items-center gap-1">
          HTTP API
          <Tooltip tip="This function was called through the Convex HTTP API.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      );
    case "HttpEndpoint":
      return (
        <div className="flex items-center gap-1">
          HTTP Endpoint
          <Tooltip tip="This HTTP Action was called by an HTTP request.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      );
    case "SyncWorker":
      return (
        <div className="flex items-center gap-1">
          Websocket
          <Tooltip tip="This function was called through a websocket connection.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      );
    case "Cron":
      return (
        <div className="flex items-center gap-1">
          Cron Job
          <Tooltip tip="This function was called by a scheduled Cron Job.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      );
    case "Scheduler":
      return (
        <div className="flex items-center gap-1">
          Scheduler
          <Tooltip tip="This function was called by a scheduled job.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      );
    case "Action":
      return (
        <div className="flex items-center gap-1">
          Action
          <Tooltip tip="This function was called by an action.">
            <QuestionMarkCircledIcon className="text-content-tertiary" />
          </Tooltip>
        </div>
      );
    default:
      return <Running />;
  }
}

function FunctionType({
  udfType,
  cachedResult,
}: {
  udfType?: string;
  cachedResult?: boolean;
}) {
  const getTypeDisplay = () => {
    switch (udfType) {
      case "Query":
        return cachedResult ? "Query (cached)" : "Query";
      case "Mutation":
        return "Mutation";
      case "Action":
        return "Action";
      case "HttpAction":
        return "HTTP Action";
      default:
        return <Running />;
    }
  };

  return <span>{getTypeDisplay()}</span>;
}

function RequestInfoList({
  outcomeNode,
  requestId,
}: {
  outcomeNode: OutcomeNode | null;
  requestId: string;
}) {
  return (
    <ul className="divide-y">
      <li className="grid grid-cols-2 items-center gap-2 py-1.5">
        <span className="text-content-secondary">Request ID</span>
        <span className="truncate font-mono text-content-primary">
          {requestId}
        </span>
      </li>
      <li className="grid grid-cols-2 items-center gap-2 py-1.5">
        <span className="text-content-secondary">Started at</span>
        <span
          className={
            outcomeNode?.endTime && outcomeNode?.executionTime
              ? "truncate text-content-primary"
              : "truncate text-content-tertiary"
          }
        >
          {outcomeNode?.endTime && outcomeNode?.executionTime ? (
            new Date(
              outcomeNode.endTime - outcomeNode.executionTime,
            ).toLocaleString()
          ) : (
            <Running />
          )}
        </span>
      </li>
      <li className="grid grid-cols-2 items-center gap-2 py-1.5">
        <span className="text-content-secondary">Completed at</span>
        <span
          className={
            outcomeNode?.endTime
              ? "truncate text-content-primary"
              : "truncate text-content-tertiary"
          }
        >
          {outcomeNode?.endTime ? (
            new Date(outcomeNode.endTime).toLocaleString()
          ) : (
            <Running />
          )}
        </span>
      </li>
      <li className="grid grid-cols-2 items-center gap-2 py-1.5">
        <span className="text-content-secondary">Duration</span>
        <span
          className={
            outcomeNode?.executionTime
              ? "flex items-center gap-1 text-content-primary"
              : "flex items-center gap-1 text-content-tertiary"
          }
        >
          {outcomeNode?.executionTime ? (
            msFormat(outcomeNode.executionTime)
          ) : (
            <Running />
          )}
        </span>
      </li>
      <li className="grid grid-cols-2 items-center gap-2 py-1.5">
        <span className="text-content-secondary">Identity</span>
        <span className="truncate text-content-primary">
          <FunctionIdentity
            identity={outcomeNode?.identityType}
            caller={outcomeNode?.caller}
          />
        </span>
      </li>
      <li className="grid grid-cols-2 items-center gap-2 py-1.5">
        <span className="text-content-secondary">Caller</span>
        <span className="truncate text-content-primary">
          <FunctionCaller caller={outcomeNode?.caller} />
        </span>
      </li>
      <li className="grid grid-cols-2 items-center gap-2 py-1.5">
        <span className="text-content-secondary">Environment</span>
        <span className="truncate text-content-primary">
          <FunctionEnvironment environment={outcomeNode?.environment} />
        </span>
      </li>
    </ul>
  );
}

function ExecutionInfoList({
  outcomeNode,
  executionId,
}: {
  outcomeNode: OutcomeNode | null;
  executionId?: string;
}) {
  const duration =
    typeof outcomeNode?.executionTime === "number"
      ? msFormat(outcomeNode.executionTime)
      : undefined;

  return (
    <ul className="divide-y">
      <li className="grid min-w-fit grid-cols-2 items-center gap-2 py-1.5">
        <span className="text-content-secondary">Execution ID</span>
        <span className="min-w-0 truncate font-mono text-content-primary">
          {executionId}
        </span>
      </li>
      <li className="grid min-w-fit grid-cols-2 items-center gap-2 py-1.5">
        <span className="text-content-secondary">Function</span>
        <span className="min-w-0 truncate">
          {outcomeNode?.functionName ? (
            <span className="font-mono text-content-primary">
              <FunctionNameOption label={outcomeNode.functionName} />
            </span>
          ) : (
            <Running />
          )}
        </span>
      </li>
      <li className="grid min-w-fit grid-cols-2 items-center gap-2 py-1.5">
        <span className="text-content-secondary">Type</span>
        <span className="min-w-0 truncate text-content-primary">
          <FunctionType
            udfType={outcomeNode?.udfType}
            cachedResult={outcomeNode?.cachedResult}
          />
        </span>
      </li>
      <li className="grid min-w-fit grid-cols-2 items-center gap-2 py-1.5">
        <span className="text-content-secondary">Started at</span>
        <span
          className={
            outcomeNode?.endTime && outcomeNode?.executionTime
              ? "truncate text-content-primary"
              : "truncate text-content-tertiary"
          }
        >
          {outcomeNode?.endTime && outcomeNode?.executionTime ? (
            new Date(
              outcomeNode.endTime - outcomeNode.executionTime,
            ).toLocaleString()
          ) : (
            <Running />
          )}
        </span>
      </li>
      <li className="grid min-w-fit grid-cols-2 items-center gap-2 py-1.5">
        <span className="text-content-secondary">Completed at</span>
        <span
          className={
            outcomeNode?.endTime
              ? "truncate text-content-primary"
              : "truncate text-content-tertiary"
          }
        >
          {outcomeNode?.endTime ? (
            new Date(outcomeNode.endTime).toLocaleString()
          ) : (
            <Running />
          )}
        </span>
      </li>
      <li className="grid min-w-fit grid-cols-2 items-center gap-2 py-1.5">
        <span className="text-content-secondary">Duration</span>
        <span
          className={
            duration
              ? "flex min-w-0 items-center gap-1 text-content-primary"
              : "flex min-w-0 items-center gap-1 text-content-tertiary"
          }
        >
          {duration || <Running />}
        </span>
      </li>
      <li className="grid min-w-fit grid-cols-2 items-center gap-2 py-1.5">
        <span className="text-content-secondary">Environment</span>
        <span className="truncate text-content-primary">
          <FunctionEnvironment environment={outcomeNode?.environment} />
        </span>
      </li>
    </ul>
  );
}

function Running() {
  return (
    <span className="flex animate-fadeInFromLoading items-center gap-1 text-content-tertiary">
      <Spinner className="ml-0 size-3" />
      Running...
    </span>
  );
}
