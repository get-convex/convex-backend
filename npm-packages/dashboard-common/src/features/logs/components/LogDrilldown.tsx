import {
  KeyboardIcon,
  Crosshair2Icon,
  InfoCircledIcon,
  ChevronDownIcon,
  ChevronUpIcon,
} from "@radix-ui/react-icons";
import { useCallback, useRef, useState } from "react";
import { Tab as HeadlessTab, Disclosure } from "@headlessui/react";
import { MAX_LOGS, UdfLog } from "@common/lib/useLogs";
import { ClosePanelButton } from "@ui/ClosePanelButton";
import { Button } from "@ui/Button";
import { LiveTimestampDistance } from "@common/elements/TimestampDistance";
import { LogLevel } from "@common/elements/LogLevel";
import { LogStatusLine } from "@common/features/logs/components/LogStatusLine";
import { LogOutput, messagesToString } from "@common/elements/LogOutput";
import { CopyButton } from "@common/elements/CopyButton";
import { DeploymentEventContent } from "@common/elements/DeploymentEventContent";
import { Tab } from "@ui/Tab";
import Link from "next/link";
import { useHotkeys } from "react-hotkeys-hook";
import { KeyboardShortcut } from "@ui/KeyboardShortcut";
import { Callout } from "@ui/Callout";
import { ITEM_SIZE } from "@common/features/logs/components/LogListItem";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";
import { cn } from "@ui/cn";
import { FunctionCallTree } from "./FunctionCallTree";
import { LogMetadata } from "./LogMetadata";
import { InterleavedLog, getTimestamp, getLogKey } from "../lib/interleaveLogs";

export function LogDrilldown({
  requestId,
  onClose,
  selectedLog: selectedLogProp,
  onFilterByRequestId,
  onSelectLog,
  onHitBoundary,
  shownInterleavedLogs,
  allUdfLogs,
  logListContainerRef,
}: {
  requestId?: string;
  shownInterleavedLogs: InterleavedLog[];
  allUdfLogs: UdfLog[];
  onClose: () => void;
  selectedLog: InterleavedLog;
  onFilterByRequestId?: (requestId: string) => void;
  onSelectLog: (log: InterleavedLog) => void;
  onHitBoundary: (boundary: "top" | "bottom" | null) => void;
  logListContainerRef?: React.RefObject<HTMLDivElement>;
}) {
  const [selectedTabIndex, setSelectedTabIndex] = useState(0);
  const tabGroupRef = useRef<HTMLDivElement>(null);
  const rightPanelRef = useRef<HTMLDivElement>(null);

  const selectedLog = selectedLogProp;

  useNavigateLogs(
    selectedLog,
    shownInterleavedLogs,
    onSelectLog,
    onClose,
    onHitBoundary,
    rightPanelRef,
    logListContainerRef,
  );

  if (!selectedLog) {
    return null;
  }

  if (selectedLog.kind === "ClearedLogs") {
    return <div className="h-full w-full border-l bg-background-primary/70" />;
  }

  return (
    <div className="flex h-full max-h-full flex-col overflow-hidden border-l bg-background-primary/70">
      {/* Header */}
      <div className="border-b bg-background-secondary px-2 pt-4 pb-4">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <h4 className="flex flex-wrap items-center gap-2">
            <div className="flex flex-wrap items-center gap-2">
              {selectedLog.kind === "ExecutionLog" ? (
                <>
                  <span className="font-mono text-sm">
                    {selectedLog.executionLog.localizedTimestamp}
                    <span className="text-content-secondary">
                      .
                      {new Date(getTimestamp(selectedLog))
                        .toISOString()
                        .split(".")[1]
                        .slice(0, -1)}
                    </span>
                  </span>
                  <span className="text-xs font-normal text-nowrap text-content-secondary">
                    (
                    <LiveTimestampDistance
                      date={new Date(getTimestamp(selectedLog))}
                      className="inline"
                    />
                    )
                  </span>
                  <div className="font-mono text-xs">
                    {selectedLog.executionLog.kind === "log" &&
                      selectedLog.executionLog.output.level && (
                        <LogLevel
                          level={selectedLog.executionLog.output.level}
                        />
                      )}
                    {selectedLog.executionLog.kind === "outcome" && (
                      <LogStatusLine
                        outcome={selectedLog.executionLog.outcome}
                      />
                    )}
                  </div>
                </>
              ) : (
                <>
                  <span className="font-mono text-sm">
                    {new Date(getTimestamp(selectedLog)).toLocaleString()}
                  </span>
                  <span className="text-xs font-normal text-nowrap text-content-secondary">
                    (
                    <LiveTimestampDistance
                      date={new Date(getTimestamp(selectedLog))}
                      className="inline"
                    />
                    )
                  </span>
                </>
              )}
            </div>
          </h4>
          <div className="flex items-center gap-1">
            {selectedLog.kind === "ExecutionLog" && requestId && (
              <Button
                icon={<Crosshair2Icon />}
                variant="neutral"
                size="xs"
                inline
                tip="View log line in context"
                onClick={() => {
                  onFilterByRequestId?.(requestId);
                }}
              />
            )}
            <ClosePanelButton onClose={onClose} />
          </div>
        </div>
      </div>

      <div
        ref={rightPanelRef}
        tabIndex={-1}
        className="scrollbar grow animate-fadeInFromLoading gap-2 overflow-y-auto py-2"
      >
        {/* Deployment Event Content */}
        {selectedLog.kind === "DeploymentEvent" && (
          <div className="m-2 mt-0 animate-fadeInFromLoading rounded-md border bg-background-secondary">
            <div className="px-2 pt-2 pb-2">
              <p className="mb-1 text-xs font-semibold">Deployment Event</p>
              <DeploymentEventContent
                event={selectedLog.deploymentEvent}
                inPanel
              />
            </div>
          </div>
        )}

        {/* Selected Log Output - show when a log is selected and it's not a completion */}
        {selectedLog.kind === "ExecutionLog" &&
          selectedLog.executionLog.kind === "log" && (
            <div
              className={cn(
                "m-2 mt-0 animate-fadeInFromLoading rounded-md border",
                selectedLog.executionLog.output.level === "ERROR"
                  ? "bg-background-error text-content-error"
                  : "bg-background-secondary",
              )}
            >
              <div className="mb-1 flex items-center justify-between gap-1 px-2 pt-2">
                <p className="text-xs font-semibold">Log Message</p>
                <CopyButton
                  text={`${messagesToString(selectedLog.executionLog.output)}${selectedLog.executionLog.output.isTruncated ? " (truncated due to length)" : ""}`}
                  className={
                    selectedLog.executionLog.output.level === "ERROR"
                      ? "text-content-error"
                      : ""
                  }
                  inline
                />
              </div>
              <div className="scrollbar max-h-60 overflow-y-auto px-2 pb-2 font-mono text-xs">
                <LogOutput output={selectedLog.executionLog.output} wrap />
              </div>
            </div>
          )}

        {/* Error message for outcome logs with errors */}
        {selectedLog.kind === "ExecutionLog" &&
          selectedLog.executionLog.kind === "outcome" &&
          selectedLog.executionLog.error && (
            <div className="m-2 mt-0 animate-fadeInFromLoading rounded-md border bg-background-error text-content-error">
              <div className="mb-1 flex items-center justify-between gap-1 px-2 pt-2">
                <p className="text-xs font-semibold">Error</p>
                <CopyButton
                  text={selectedLog.executionLog.error}
                  inline
                  className="text-content-error"
                />
              </div>
              <div className="px-2 pb-2 font-mono text-xs">
                <LogOutput
                  output={{
                    isTruncated: false,
                    messages: [selectedLog.executionLog.error],
                    level: "FAILURE",
                  }}
                  wrap
                />
              </div>
            </div>
          )}
        {selectedLog.kind === "ExecutionLog" && allUdfLogs.length === 0 && (
          <Callout
            className="mx-2 mb-2 flex items-center gap-2 p-1.5 text-xs"
            variant="upsell"
          >
            <InfoCircledIcon className="min-w-4" />
            <div>
              Heads up! The logs page only keeps track of the latest{" "}
              {MAX_LOGS.toLocaleString()} logs. This panel is missing
              information for this log because it is older than the maximum
              number of logs. If you need log persistence, try out{" "}
              <Link
                href="https://docs.convex.dev/production/integrations/log-streams/"
                className="text-content-link hover:underline"
                target="_blank"
              >
                Log Streams
              </Link>
              .
            </div>
          </Callout>
        )}

        {/* Tabs for Execution Info, Request Info, and Functions Called - only for ExecutionLog */}
        {selectedLog.kind === "ExecutionLog" && requestId && (
          <div className="relative flex grow flex-col">
            <HeadlessTab.Group
              selectedIndex={selectedTabIndex}
              onChange={setSelectedTabIndex}
            >
              <div className="sticky top-0 z-10 px-2" ref={tabGroupRef}>
                <HeadlessTab.List className="flex gap-1 rounded-t-md border bg-background-secondary px-1">
                  <Tab>Execution</Tab>
                  <Tab>Request</Tab>
                  <Tab>Functions Called</Tab>
                </HeadlessTab.List>
              </div>

              <div className="mx-2 flex h-fit max-h-full min-h-0 flex-col rounded rounded-t-none border border-t-0 bg-background-secondary">
                <div className="flex flex-col gap-2">
                  <HeadlessTab.Panels>
                    <HeadlessTab.Panel>
                      <LogMetadata
                        requestId={requestId}
                        logs={allUdfLogs}
                        executionId={selectedLog.executionLog.executionId}
                      />
                    </HeadlessTab.Panel>
                    <HeadlessTab.Panel>
                      <LogMetadata
                        requestId={requestId}
                        logs={allUdfLogs}
                        executionId={undefined}
                      />
                    </HeadlessTab.Panel>
                    <HeadlessTab.Panel>
                      <FunctionCallTree
                        logs={allUdfLogs}
                        currentLog={selectedLog.executionLog}
                      />
                    </HeadlessTab.Panel>
                  </HeadlessTab.Panels>
                </div>
              </div>
            </HeadlessTab.Group>
          </div>
        )}
      </div>

      <KeyboardShortcutsSection selectedLog={selectedLog} />
    </div>
  );
}

const shortcutItemClass = "grid grid-cols-[1fr_9.5rem] gap-x-2 min-w-0";
const shortcutKeysClass = "flex items-center justify-end gap-1";
const shortcutLabelClass = "truncate min-w-0";

function KeyboardShortcutsSection({
  selectedLog,
}: {
  selectedLog: InterleavedLog;
}) {
  const [isOpen, setIsOpen] = useGlobalLocalStorage(
    "logDrilldown.keyboardShortcuts.open",
    false,
  );

  return (
    <section className="border-t bg-background-tertiary px-4 py-2">
      <Disclosure defaultOpen={isOpen}>
        {({ open }) => (
          <>
            <div className="flex items-center justify-between">
              <Disclosure.Button
                className="flex items-center gap-1 text-xs"
                onClick={() => setIsOpen(!isOpen)}
              >
                <KeyboardIcon className="relative -left-0.5 text-content-secondary" />
                <h6 className="font-semibold text-content-secondary">
                  Keyboard Shortcuts
                </h6>
                {open ? (
                  <ChevronUpIcon className="size-3" />
                ) : (
                  <ChevronDownIcon className="size-3" />
                )}
              </Disclosure.Button>
            </div>

            <Disclosure.Panel className="mt-2 scrollbar animate-fadeInFromLoading overflow-x-auto">
              <div className="grid grid-cols-[16.5rem_14rem] gap-x-4 gap-y-1 text-xs text-content-secondary">
                <div className={shortcutItemClass}>
                  <div className={shortcutKeysClass}>
                    <KeyboardShortcut value={["Down"]} />
                    <span>/</span>
                    <KeyboardShortcut value={["Up"]} />
                  </div>
                  <span className={shortcutLabelClass}>Navigate</span>
                </div>

                <div className={shortcutItemClass}>
                  <div className={shortcutKeysClass}>
                    <KeyboardShortcut value={["CtrlOrCmd"]} />
                    <span>+</span>
                    <KeyboardShortcut value={["A"]} />
                  </div>
                  <span className={shortcutLabelClass}>Jump to top</span>
                </div>

                {selectedLog.kind === "ExecutionLog" && (
                  <>
                    <div className={shortcutItemClass}>
                      <div className={shortcutKeysClass}>
                        <KeyboardShortcut value={["Shift"]} />
                        <span>+</span>
                        <KeyboardShortcut value={["Down"]} />
                        <span>/</span>
                        <KeyboardShortcut value={["Up"]} />
                      </div>
                      <span className={shortcutLabelClass}>
                        Navigate request
                      </span>
                    </div>

                    <div className={shortcutItemClass}>
                      <div className={shortcutKeysClass}>
                        <KeyboardShortcut value={["CtrlOrCmd"]} />
                        <span>+</span>
                        <KeyboardShortcut value={["E"]} />
                      </div>
                      <span className={shortcutLabelClass}>Jump to bottom</span>
                    </div>

                    <div className={shortcutItemClass}>
                      <div className={shortcutKeysClass}>
                        <KeyboardShortcut value={["CtrlOrCmd"]} />
                        <span>+</span>
                        <KeyboardShortcut value={["Down"]} />
                        <span>/</span>
                        <KeyboardShortcut value={["Up"]} />
                      </div>
                      <span className={shortcutLabelClass}>
                        Navigate execution
                      </span>
                    </div>

                    <div className={shortcutItemClass}>
                      <div className={shortcutKeysClass}>
                        <KeyboardShortcut value={["CtrlOrCmd"]} />
                        <KeyboardShortcut value={["Shift"]} />
                        <span>+</span>
                        <KeyboardShortcut value={["A"]} />
                      </div>
                      <span className={shortcutLabelClass}>
                        Jump to top of request
                      </span>
                    </div>
                  </>
                )}

                <div className={shortcutItemClass}>
                  <div className={shortcutKeysClass}>
                    <KeyboardShortcut value={["CtrlOrCmd"]} />
                    <span>+</span>
                    <KeyboardShortcut value={["PageUp"]} />
                    <span>/</span>
                    <KeyboardShortcut value={["PageDown"]} />
                  </div>
                  <span className={shortcutLabelClass}>Navigate page</span>
                </div>

                {selectedLog.kind === "ExecutionLog" ? (
                  <div className={shortcutItemClass}>
                    <div className={shortcutKeysClass}>
                      <KeyboardShortcut value={["CtrlOrCmd"]} />
                      <KeyboardShortcut value={["Shift"]} />
                      <span>+</span>
                      <KeyboardShortcut value={["E"]} />
                    </div>
                    <span className={shortcutLabelClass}>
                      Jump to bottom of request
                    </span>
                  </div>
                ) : (
                  <div className={shortcutItemClass}>
                    <div className={shortcutKeysClass}>
                      <KeyboardShortcut value={["CtrlOrCmd"]} />
                      <span>+</span>
                      <KeyboardShortcut value={["E"]} />
                    </div>
                    <span className={shortcutLabelClass}>Jump to bottom</span>
                  </div>
                )}

                <div className={shortcutItemClass}>
                  <div className={shortcutKeysClass}>
                    <KeyboardShortcut value={["Shift"]} />
                    <span>+</span>
                    <KeyboardShortcut value={["Right"]} />
                  </div>
                  <span className={shortcutLabelClass}>Focus this panel</span>
                </div>

                <div className={shortcutItemClass}>
                  <div className={shortcutKeysClass}>
                    <KeyboardShortcut value={["Esc"]} />
                  </div>
                  <span className={shortcutLabelClass}>Close this panel</span>
                </div>
              </div>
            </Disclosure.Panel>
          </>
        )}
      </Disclosure>
    </section>
  );
}

export function useNavigateLogs(
  selectedLog: InterleavedLog | null,
  logs: InterleavedLog[],
  onSelectLog: (log: InterleavedLog) => void,
  onClose: () => void,
  onHitBoundary: (boundary: "top" | "bottom" | null) => void,
  rightPanelRef: React.RefObject<HTMLDivElement>,
  logListContainerRef?: React.RefObject<HTMLDivElement>,
) {
  // Calculate the number of items that fit in one page based on container height
  const calculatePageSize = useCallback(() => {
    if (!logListContainerRef?.current) {
      return 10; // Default fallback
    }
    const containerHeight = logListContainerRef.current.clientHeight;
    return Math.floor(containerHeight / ITEM_SIZE);
  }, [logListContainerRef]);
  // Get logs for the current execution (both log entries and outcomes)
  const executionLogs =
    selectedLog && selectedLog.kind === "ExecutionLog"
      ? logs.filter(
          (log) =>
            log.kind === "ExecutionLog" &&
            log.executionLog.executionId ===
              selectedLog.executionLog.executionId,
        )
      : undefined;

  // Get logs for the current request (both log entries and outcomes)
  const requestLogs =
    selectedLog && selectedLog.kind === "ExecutionLog"
      ? logs.filter(
          (log) =>
            log.kind === "ExecutionLog" &&
            log.executionLog.requestId === selectedLog.executionLog.requestId,
        )
      : undefined;

  // Navigate to prev/next log
  const navigateLog = useCallback(
    (direction: "prev" | "next", scope: "execution" | "request" | "all") => {
      if (!selectedLog || !onSelectLog) return;

      let targetLogs: InterleavedLog[] | undefined;
      if (scope === "execution") {
        targetLogs = executionLogs;
      } else if (scope === "request") {
        targetLogs = requestLogs;
      } else {
        targetLogs = logs;
      }

      if (!targetLogs) return;

      const selectedLogKey = getLogKey(selectedLog);
      const currentIndex = targetLogs.findIndex(
        (log) => getLogKey(log) === selectedLogKey,
      );

      if (currentIndex === -1) {
        console.warn("Current log not found in target logs", {
          selectedLog,
          targetLogsCount: targetLogs.length,
          scope,
        });
        return;
      }

      const nextIndex =
        direction === "prev" ? currentIndex - 1 : currentIndex + 1;
      if (nextIndex >= 0 && nextIndex < targetLogs.length) {
        onSelectLog(targetLogs[nextIndex]);
        onHitBoundary(null);
      } else {
        // Hit the boundary - trigger animation
        onHitBoundary(direction === "next" ? "bottom" : "top");
      }
    },
    [selectedLog, executionLogs, requestLogs, logs, onSelectLog, onHitBoundary],
  );

  // Keyboard shortcuts
  useHotkeys("down", () => navigateLog("next", "all"), {
    preventDefault: true,
  });
  useHotkeys("up", () => navigateLog("prev", "all"), {
    preventDefault: true,
  });
  useHotkeys("shift+down", () => navigateLog("next", "request"), {
    preventDefault: true,
  });
  useHotkeys("shift+up", () => navigateLog("prev", "request"), {
    preventDefault: true,
  });
  useHotkeys(
    ["ctrl+down", "meta+down"],
    () => navigateLog("next", "execution"),
    {
      preventDefault: true,
    },
  );
  useHotkeys(["meta+up", "ctrl+up"], () => navigateLog("prev", "execution"), {
    preventDefault: true,
  });
  // Handle arrow key navigation between log list and right panel
  useHotkeys(
    "shift+right",
    () => {
      // Focus the right panel content area
      if (rightPanelRef.current) {
        rightPanelRef.current.focus();
      }
    },
    {
      preventDefault: true,
    },
  );

  useHotkeys(
    "esc",
    () => {
      onClose();
    },
    {
      preventDefault: true,
    },
  );

  // Navigate to top/bottom of list
  useHotkeys(
    ["ctrl+a", "meta+a"],
    () => {
      if (logs.length > 0) {
        onSelectLog(logs[0]);
        onHitBoundary(null);
      }
    },
    {
      preventDefault: true,
    },
  );
  useHotkeys(
    ["ctrl+e", "meta+e"],
    () => {
      if (logs.length > 0) {
        onSelectLog(logs[logs.length - 1]);
        onHitBoundary(null);
      }
    },
    {
      preventDefault: true,
    },
  );

  // Navigate to top/bottom within request
  useHotkeys(
    ["ctrl+shift+a", "meta+shift+a"],
    () => {
      if (requestLogs && requestLogs.length > 0) {
        onSelectLog(requestLogs[0]);
        onHitBoundary(null);
      }
    },
    {
      preventDefault: true,
    },
  );
  useHotkeys(
    ["ctrl+shift+e", "meta+shift+e"],
    () => {
      if (requestLogs && requestLogs.length > 0) {
        onSelectLog(requestLogs[requestLogs.length - 1]);
        onHitBoundary(null);
      }
    },
    {
      preventDefault: true,
    },
  );

  // Navigate by page (based on container height)
  useHotkeys(
    ["ctrl+pageup", "meta+pageup"],
    () => {
      if (!selectedLog) return;
      const pageSize = calculatePageSize();
      const selectedLogKey = getLogKey(selectedLog);
      const currentIndex = logs.findIndex(
        (log) => getLogKey(log) === selectedLogKey,
      );
      if (currentIndex === -1) return;

      const newIndex = Math.max(0, currentIndex - pageSize);
      onSelectLog(logs[newIndex]);
      if (newIndex === 0) {
        onHitBoundary("top");
      } else {
        onHitBoundary(null);
      }
    },
    {
      preventDefault: true,
    },
    [selectedLog, logs, onSelectLog, onHitBoundary, calculatePageSize],
  );
  useHotkeys(
    ["ctrl+pagedown", "meta+pagedown"],
    () => {
      if (!selectedLog) return;
      const pageSize = calculatePageSize();
      const selectedLogKey = getLogKey(selectedLog);
      const currentIndex = logs.findIndex(
        (log) => getLogKey(log) === selectedLogKey,
      );
      if (currentIndex === -1) return;

      const newIndex = Math.min(logs.length - 1, currentIndex + pageSize);
      onSelectLog(logs[newIndex]);
      if (newIndex === logs.length - 1) {
        onHitBoundary("bottom");
      } else {
        onHitBoundary(null);
      }
    },
    {
      preventDefault: true,
    },
    [selectedLog, logs, onSelectLog, onHitBoundary, calculatePageSize],
  );
}
