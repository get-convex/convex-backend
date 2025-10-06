import { Crosshair2Icon } from "@radix-ui/react-icons";
import { useCallback, useMemo, useRef, useState } from "react";
import { Tab as HeadlessTab } from "@headlessui/react";
import { UdfLog } from "@common/lib/useLogs";
import { ClosePanelButton } from "@ui/ClosePanelButton";
import { Button } from "@ui/Button";
import { LiveTimestampDistance } from "@common/elements/TimestampDistance";
import { LogLevel } from "@common/elements/LogLevel";
import { LogStatusLine } from "@common/features/logs/components/LogStatusLine";
import { LogOutput, messagesToString } from "@common/elements/LogOutput";
import { CopyButton } from "@common/elements/CopyButton";
import { Tab } from "@ui/Tab";
import { useHotkeys } from "react-hotkeys-hook";
import { KeyboardShortcut } from "@ui/KeyboardShortcut";
import { FunctionCallTree } from "./FunctionCallTree";
import { LogMetadata } from "./LogMetadata";

export function LogDrilldown({
  requestId,
  logs,
  onClose,
  selectedLogTimestamp,
  onFilterByRequestId,
  onSelectLog,
  onHitBoundary,
}: {
  requestId: string;
  logs: UdfLog[];
  onClose: () => void;
  selectedLogTimestamp?: number;
  onFilterByRequestId?: (requestId: string) => void;
  onSelectLog: (timestamp: number) => void;
  onHitBoundary: (boundary: "top" | "bottom" | null) => void;
}) {
  const [selectedTabIndex, setSelectedTabIndex] = useState(0);
  const tabGroupRef = useRef<HTMLDivElement>(null);
  const rightPanelRef = useRef<HTMLDivElement>(null);

  // Get the selected log to display based on timestamp
  const selectedLog = useMemo(() => {
    if (selectedLogTimestamp !== undefined) {
      return logs.find((log) => log.timestamp === selectedLogTimestamp) || null;
    }
    return null;
  }, [logs, selectedLogTimestamp]);

  useNavigateLogs(selectedLog, logs, onSelectLog, onHitBoundary, rightPanelRef);

  if (!selectedLog) {
    return null;
  }

  return (
    <div className="flex h-full max-h-full flex-col overflow-hidden border-l bg-background-primary/70">
      {/* Header */}
      <div className="border-b bg-background-secondary px-2 pt-4 pb-4">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <h4 className="flex flex-wrap items-center gap-2">
            <div className="flex flex-wrap items-center gap-2">
              <span className="font-mono text-sm">
                {selectedLog.localizedTimestamp}
                <span className="text-content-secondary">
                  .
                  {new Date(selectedLog.timestamp)
                    .toISOString()
                    .split(".")[1]
                    .slice(0, -1)}
                </span>
              </span>
              <span className="text-xs font-normal text-nowrap text-content-secondary">
                (
                <LiveTimestampDistance
                  date={new Date(selectedLog.timestamp)}
                  className="inline"
                />
                )
              </span>
              <div className="font-mono text-xs">
                {selectedLog.kind === "log" && selectedLog.output.level && (
                  <LogLevel level={selectedLog.output.level} />
                )}
                {selectedLog.kind === "outcome" && (
                  <LogStatusLine outcome={selectedLog.outcome} />
                )}
              </div>
            </div>
          </h4>
          <div className="flex items-center gap-1">
            {selectedLog && (
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
        {/* Selected Log Output - show when a log is selected and it's not a completion */}
        {selectedLog && selectedLog.kind === "log" && (
          <div className="m-2 mt-0 animate-fadeInFromLoading rounded-md border bg-background-secondary">
            <div className="mb-1 flex items-center justify-between gap-1 px-2 pt-2">
              <p className="text-xs font-semibold">Log Message</p>
              <CopyButton
                text={`${messagesToString(selectedLog.output)}${selectedLog.output.isTruncated ? " (truncated due to length)" : ""}`}
                inline
              />
            </div>
            <div className="px-2 pb-2 font-mono text-xs">
              <LogOutput output={selectedLog.output} wrap />
            </div>
          </div>
        )}

        {/* Error message for outcome logs with errors */}
        {selectedLog && selectedLog.kind === "outcome" && selectedLog.error && (
          <div className="m-2 mt-0 animate-fadeInFromLoading rounded-md border bg-background-secondary">
            <div className="mb-1 flex items-center justify-between gap-1 px-2 pt-2">
              <p className="text-xs font-semibold">Error</p>
              <CopyButton text={selectedLog.error} inline />
            </div>
            <div className="px-2 pb-2 font-mono text-xs">
              <LogOutput
                output={{
                  isTruncated: false,
                  messages: [selectedLog.error],
                  level: "FAILURE",
                }}
                wrap
              />
            </div>
          </div>
        )}

        {/* Tabs for Execution Info, Request Info, and Functions Called */}
        <HeadlessTab.Group
          selectedIndex={selectedTabIndex}
          onChange={setSelectedTabIndex}
        >
          <div className="px-2" ref={tabGroupRef}>
            <HeadlessTab.List className="flex gap-1 rounded-t-md border bg-background-secondary px-1">
              {selectedLog && <Tab>Execution</Tab>}
              <Tab>Request</Tab>
              <Tab>Functions Called</Tab>
            </HeadlessTab.List>
          </div>

          <div className="mx-2 scrollbar flex h-fit min-h-0 flex-col overflow-y-auto rounded rounded-t-none border border-t-0 bg-background-secondary">
            <div className="flex flex-col gap-2">
              <HeadlessTab.Panels>
                {selectedLog && (
                  <HeadlessTab.Panel>
                    <LogMetadata
                      requestId={requestId}
                      logs={logs}
                      executionId={selectedLog.executionId}
                    />
                  </HeadlessTab.Panel>
                )}
                <HeadlessTab.Panel>
                  <LogMetadata
                    requestId={requestId}
                    logs={logs}
                    executionId={undefined}
                  />
                </HeadlessTab.Panel>
                <HeadlessTab.Panel>
                  <FunctionCallTree
                    logs={logs.filter((log) => log.requestId === requestId)}
                    onFunctionSelect={(executionId) => {
                      // Find the outcome log for this execution
                      const outcomeLog = logs.find(
                        (log) =>
                          log.kind === "outcome" &&
                          log.executionId === executionId,
                      );
                      if (outcomeLog && onSelectLog) {
                        onSelectLog(outcomeLog.timestamp);
                      }
                    }}
                  />
                </HeadlessTab.Panel>
              </HeadlessTab.Panels>
            </div>
          </div>
        </HeadlessTab.Group>
      </div>

      <KeyboardShortcutsSection />
    </div>
  );
}

function KeyboardShortcutsSection() {
  return (
    <section className="border-t bg-background-tertiary px-4 py-2">
      <div className="grid grid-cols-[auto_1fr] gap-x-1 gap-y-1 text-xs text-content-secondary">
        <div className="flex items-center justify-end gap-1">
          <KeyboardShortcut value={["Down"]} />
          <span>/</span>
          <KeyboardShortcut value={["Up"]} />
        </div>
        <span>Navigate</span>

        <div className="flex items-center justify-end gap-1">
          <KeyboardShortcut value={["Shift"]} />
          <span>+</span>
          <KeyboardShortcut value={["Down"]} />
          <span>/</span>
          <KeyboardShortcut value={["Up"]} />
        </div>
        <span>Navigate within request</span>

        <div className="flex items-center justify-end gap-1">
          <KeyboardShortcut value={["CtrlOrCmd"]} />
          <span>+</span>
          <KeyboardShortcut value={["Down"]} />
          <span>/</span>
          <KeyboardShortcut value={["Up"]} />
        </div>
        <span>Navigate within execution</span>

        <div className="flex items-center justify-end gap-1">
          <KeyboardShortcut value={["Shift"]} />
          <span>+</span>
          <KeyboardShortcut value={["Right"]} />
        </div>
        <span>Focus this panel</span>
      </div>
    </section>
  );
}

export function useNavigateLogs(
  selectedLog: UdfLog | null,
  logs: UdfLog[],
  onSelectLog: (timestamp: number) => void,
  onHitBoundary: (boundary: "top" | "bottom" | null) => void,
  rightPanelRef: React.RefObject<HTMLDivElement>,
) {
  // Get logs for the current execution (both log entries and outcomes)
  const executionLogs = selectedLog
    ? logs.filter((log) => log.executionId === selectedLog.executionId)
    : undefined;

  // Get logs for the current request (both log entries and outcomes)
  const requestLogs = selectedLog
    ? logs.filter((log) => log.requestId === selectedLog.requestId)
    : undefined;

  // Navigate to prev/next log
  const navigateLog = useCallback(
    (direction: "prev" | "next", scope: "execution" | "request" | "all") => {
      if (!selectedLog || !onSelectLog) return;

      let targetLogs: UdfLog[] | undefined;
      if (scope === "execution") {
        targetLogs = executionLogs;
      } else if (scope === "request") {
        targetLogs = requestLogs;
      } else {
        targetLogs = logs;
      }

      if (!targetLogs) return;

      // Sort logs by timestamp to ensure correct order
      const sortedLogs = [...targetLogs].sort(
        (a, b) => a.timestamp - b.timestamp,
      );

      const currentIndex = sortedLogs.findIndex(
        (log) => log.timestamp === selectedLog.timestamp,
      );

      if (currentIndex === -1) {
        console.warn("Current log not found in target logs", {
          selectedLog,
          targetLogsCount: sortedLogs.length,
          scope,
        });
        return;
      }

      const nextIndex =
        direction === "next" ? currentIndex - 1 : currentIndex + 1;
      if (nextIndex >= 0 && nextIndex < sortedLogs.length) {
        onSelectLog(sortedLogs[nextIndex].timestamp);
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
}
