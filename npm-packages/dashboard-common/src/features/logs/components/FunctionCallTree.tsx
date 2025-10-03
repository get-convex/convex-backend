import {
  CaretRightIcon,
  CheckCircledIcon,
  CrossCircledIcon,
} from "@radix-ui/react-icons";
import { useMemo } from "react";
import { UdfLog, UdfLogOutcome } from "@common/lib/useLogs";
import { FunctionNameOption } from "@common/elements/FunctionNameOption";
import { Button } from "@ui/Button";
import { msFormat } from "@common/lib/format";
import { Spinner } from "@ui/Spinner";

type ExecutionNode = {
  executionId: string;
  functionName: string;
  startTime: number;
  executionTime?: number;
  status: "success" | "failure" | "running";
  parentExecutionId?: string | null;
  caller?: string;
  environment?: string;
  identityType?: string;
  children: ExecutionNode[];
  error?: string;
  logCount: number;
  udfType: string;
};

export function FunctionCallTree({
  logs,
  onFunctionSelect,
}: {
  logs: UdfLog[];
  onFunctionSelect: (executionId: string, functionName: string) => void;
}) {
  const executionNodes = useMemo(() => createExecutionNodes(logs), [logs]);

  if (executionNodes.length === 0) {
    return null;
  }

  const rootNode = executionNodes[0];

  return (
    <div className="scrollbar overflow-auto p-2 text-xs">
      {rootNode && (
        <div>
          {executionNodes.map((node) => (
            <ExecutionTreeNode
              key={node.executionId}
              node={node}
              level={0}
              onFunctionSelect={onFunctionSelect}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function ExecutionTreeNode({
  node,
  level,
  onFunctionSelect,
}: {
  node: ExecutionNode;
  level: number;
  onFunctionSelect: (executionId: string, functionName: string) => void;
}) {
  const hasChildren = node.children.length > 0;

  return (
    <div className="font-mono text-xs">
      <Button
        className="flex h-[30px] w-full cursor-pointer items-center rounded-md pr-2 pl-4 hover:bg-background-tertiary"
        variant="unstyled"
        onClick={() => onFunctionSelect(node.executionId, node.functionName)}
      >
        <div className="flex h-full items-center">
          {level !== 0 &&
            Array.from({ length: level }).map((_, index) => (
              <div
                key={index}
                className="mr-4 h-full w-[1px] shrink-0 bg-border-transparent"
              />
            ))}
        </div>

        <div className="-ml-1.5 flex items-center gap-1">
          {node.status === "running" ? (
            <Spinner className="size-4" />
          ) : node.error ? (
            <CrossCircledIcon
              className="size-4 text-content-error"
              aria-label="Function call failed"
            />
          ) : node.status === "success" ? (
            <CheckCircledIcon
              className="size-4 text-content-success"
              aria-label="Function call succeeded"
            />
          ) : null}

          <FunctionNameOption label={node.functionName} error={!!node.error} />
        </div>
        {node.executionTime && (
          <span className="ml-1 text-content-secondary">
            ({msFormat(node.executionTime)})
          </span>
        )}
        <div className="ml-auto flex items-center gap-1">
          {node.logCount > 0 && (
            <span className="font-sans text-xs text-content-secondary">
              {node.logCount} log{node.logCount === 1 ? "" : "s"}
            </span>
          )}
          <CaretRightIcon />
        </div>
      </Button>

      {hasChildren && (
        <div>
          {node.children.map((child) => (
            <ExecutionTreeNode
              key={child.executionId}
              node={child}
              level={level + 1}
              onFunctionSelect={onFunctionSelect}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function countLogsByExecution(logs: UdfLog[]): Map<string, number> {
  const logCountMap = new Map<string, number>();
  logs
    .filter((log) => log.kind === "log")
    .forEach((log) => {
      logCountMap.set(
        log.executionId,
        (logCountMap.get(log.executionId) || 0) + 1,
      );
    });
  return logCountMap;
}

function createCompletedExecutionNodes(
  logs: UdfLog[],
  logCountMap: Map<string, number>,
): { nodeMap: Map<string, ExecutionNode>; outcomeSet: Set<string> } {
  const nodeMap = new Map<string, ExecutionNode>();
  const outcomeSet = new Set<string>();

  logs
    .filter((log): log is UdfLog & UdfLogOutcome => log.kind === "outcome")
    .forEach((log) => {
      const node: ExecutionNode = {
        executionId: log.executionId,
        functionName: log.call,
        startTime: log.executionTimestamp || log.timestamp,
        executionTime: log.executionTimeMs ?? undefined,
        status: log.outcome.status,
        parentExecutionId: log.parentExecutionId,
        caller: log.caller,
        environment: log.environment,
        identityType: log.identityType,
        children: [],
        error: log.error,
        logCount: logCountMap.get(log.executionId) || 0,
        udfType: log.udfType,
      };

      nodeMap.set(log.executionId, node);
      outcomeSet.add(log.executionId);
    });

  return { nodeMap, outcomeSet };
}

function createRunningExecutionNodes(
  logs: UdfLog[],
  logCountMap: Map<string, number>,
  nodeMap: Map<string, ExecutionNode>,
  outcomeSet: Set<string>,
): void {
  const executionLogMap = new Map<string, UdfLog[]>();
  logs
    .filter((log) => log.kind === "log")
    .forEach((log) => {
      if (!executionLogMap.has(log.executionId)) {
        executionLogMap.set(log.executionId, []);
      }
      executionLogMap.get(log.executionId)!.push(log);
    });

  executionLogMap.forEach((logEntries, executionId) => {
    if (!outcomeSet.has(executionId)) {
      const firstLog = logEntries[0];
      // Try to find parent info from any log entry (might be in overrides)
      const { parentExecutionId } = firstLog as any;

      const node: ExecutionNode = {
        executionId: firstLog.executionId,
        functionName: firstLog.call,
        startTime: firstLog.timestamp,
        executionTime: undefined,
        status: "running",
        parentExecutionId: parentExecutionId || undefined,
        caller: undefined,
        environment: undefined,
        identityType: undefined,
        children: [],
        error: undefined,
        logCount: logCountMap.get(executionId) || 0,
        udfType: firstLog.udfType,
      };

      nodeMap.set(executionId, node);
    }
  });
}

function buildExecutionTree(
  nodeMap: Map<string, ExecutionNode>,
): ExecutionNode[] {
  const sortChildren = (node: ExecutionNode) => {
    node.children.sort((a, b) => a.startTime - b.startTime);
    node.children.forEach(sortChildren);
  };

  const rootNodes: ExecutionNode[] = [];
  const allNodes = Array.from(nodeMap.values());

  // Build the tree structure
  allNodes.forEach((node) => {
    if (!node.parentExecutionId) {
      rootNodes.push(node);
    } else {
      const parent = nodeMap.get(node.parentExecutionId);
      if (parent) {
        parent.children.push(node);
      } else {
        rootNodes.push(node);
      }
    }
  });

  rootNodes.sort((a, b) => a.startTime - b.startTime);
  rootNodes.forEach((node) => sortChildren(node));

  return rootNodes;
}

export function createExecutionNodes(logs: UdfLog[]): ExecutionNode[] {
  const logCountMap = countLogsByExecution(logs);
  const { nodeMap, outcomeSet } = createCompletedExecutionNodes(
    logs,
    logCountMap,
  );
  createRunningExecutionNodes(logs, logCountMap, nodeMap, outcomeSet);
  return buildExecutionTree(nodeMap);
}
