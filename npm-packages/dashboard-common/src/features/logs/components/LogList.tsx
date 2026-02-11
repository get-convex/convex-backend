import {
  ArrowDownIcon,
  CaretDownIcon,
  HamburgerMenuIcon,
  InfoCircledIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { FixedSizeList, ListOnScrollProps, areEqual } from "react-window";
import { useMeasure } from "react-use";
import { PauseCircleIcon, PlayCircleIcon } from "@heroicons/react/24/outline";
import { useHotkeys } from "react-hotkeys-hook";
import { DeploymentEventListItem } from "@common/features/logs/components/DeploymentEventListItem";
import {
  ITEM_SIZE,
  LogListItem,
} from "@common/features/logs/components/LogListItem";
import { UdfLog } from "@common/lib/useLogs";
import {
  InterleavedLog,
  interleaveLogs,
  getLogKey,
} from "@common/features/logs/lib/interleaveLogs";
import { DeploymentAuditLogEvent } from "@common/lib/useDeploymentAuditLog";
import { Sheet } from "@ui/Sheet";
import { Tooltip } from "@ui/Tooltip";
import { InfiniteScrollList } from "@common/elements/InfiniteScrollList";
import { Button } from "@ui/Button";
import { Panel, PanelGroup } from "react-resizable-panels";
import { cn } from "@ui/cn";
import { ResizeHandle } from "@common/layouts/SidebarDetailLayout";
import { LogDrilldown } from "./LogDrilldown";
import { useLogsClipboard } from "../lib/useLogsClipboard";

export type LogListProps = {
  logs?: UdfLog[];
  pausedLogs?: UdfLog[];
  filteredLogs?: UdfLog[];
  deploymentAuditLogs?: DeploymentAuditLogEvent[];
  setFilter?: (filter: string) => void;
  clearedLogs: number[];
  setClearedLogs: (clearedLogs: number[]) => void;
  paused: boolean;
  setPaused: (paused: boolean) => void;
  setManuallyPaused: (paused: boolean) => void;
  filter?: string;
};

/**
 * Hook to manage hit boundary state with automatic timeout reset.
 * When a boundary is hit, it will automatically reset to null after 750ms.
 */
export function useHitBoundary() {
  const [hitBoundary, setHitBoundaryState] = useState<"top" | "bottom" | null>(
    null,
  );
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);

  const setHitBoundary = useCallback((boundary: "top" | "bottom" | null) => {
    // Clear any existing timeout
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }

    setHitBoundaryState(boundary);

    // If setting a boundary (not null), schedule auto-reset
    if (boundary !== null) {
      timeoutRef.current = setTimeout(() => {
        setHitBoundaryState(null);
        timeoutRef.current = null;
      }, 750);
    }
  }, []);

  // Cleanup timeout on unmount
  useEffect(
    () => () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    },
    [],
  );

  return { hitBoundary, setHitBoundary };
}

export function LogList({
  logs,
  pausedLogs,
  filteredLogs,
  deploymentAuditLogs,
  clearedLogs,
  setClearedLogs,
  paused,
  setPaused,
  setManuallyPaused,
  setFilter,
  filter,
}: LogListProps) {
  const focusTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  const interleavedLogs = interleaveLogs(
    filteredLogs ?? [],
    deploymentAuditLogs ?? [],
    clearedLogs,
  ).reverse();

  const [sheetRef, { height: heightOfListContainer }] =
    useMeasure<HTMLDivElement>();

  // Local state for hit boundary with automatic timeout reset
  const { hitBoundary, setHitBoundary } = useHitBoundary();

  // Local state for shown log
  const [shownLog, setShownLog] = useState<InterleavedLog | undefined>(
    undefined,
  );
  const [selectedRange, setSelectedRange] = useState<{
    startIndex: number;
    endIndex: number;
  } | null>(null);

  // Ref to the virtualized list for programmatic scrolling
  const listRef = useRef<FixedSizeList>(null);

  // Ref to the outer div container for calculating page size
  const outerRef = useRef<HTMLDivElement>(null);

  const handleSelectLog = useCallback(
    (log: InterleavedLog) => {
      setShownLog(log);

      // Scroll to the log in the virtualized list
      if (listRef.current && interleavedLogs) {
        const index = interleavedLogs.findIndex(
          (l) => getLogKey(l) === getLogKey(log),
        );
        if (index !== -1) {
          listRef.current.scrollToItem(index, "smart");

          // Clear any existing focus timeout
          if (focusTimeoutRef.current) {
            clearTimeout(focusTimeoutRef.current);
            focusTimeoutRef.current = null;
          }

          // Focus the button element after scroll completes
          // Use a short timeout to allow the scroll to complete
          focusTimeoutRef.current = setTimeout(() => {
            const logKey = getLogKey(log);
            const safeLogKey =
              typeof CSS !== "undefined" && CSS.escape
                ? CSS.escape(logKey)
                : logKey.replace(/["\\]/g, "\\$&");
            const button = document.querySelector(
              `[data-log-key="${safeLogKey}"]`,
            ) as HTMLButtonElement;
            if (button && document.activeElement !== button) {
              button.focus();
              // Add a visual "ping" to the focused item to make jumps obvious
              button.classList.add("ring-2", "ring-primary", "ring-inset");
              setTimeout(() => {
                button.classList.remove("ring-2", "ring-primary", "ring-inset");
              }, 1000);
            }
          }, 100);
        }
      }
    },
    [interleavedLogs],
  );

  // Jump to Error
  const jumpToError = useCallback(
    (direction: "next" | "prev") => {
      if (!interleavedLogs) return;

      const currentIndex = shownLog
        ? interleavedLogs.findIndex((l) => getLogKey(l) === getLogKey(shownLog))
        : -1;

      const findError = (log: InterleavedLog) => {
        if (log.kind === "ExecutionLog") {
          return log.executionLog.kind === "outcome"
            ? !!log.executionLog.error
            : log.executionLog.output.level === "ERROR";
        }
        return false;
      };

      let nextIndex = -1;
      if (direction === "next") {
        nextIndex = interleavedLogs.findIndex(
          (l, i) => i > currentIndex && findError(l),
        );
      } else {
        // Reverse search for previous
        for (let i = currentIndex - 1; i >= 0; i--) {
          if (findError(interleavedLogs[i])) {
            nextIndex = i;
            break;
          }
        }
      }

      if (nextIndex !== -1) {
        handleSelectLog(interleavedLogs[nextIndex]);
      }
    },
    [interleavedLogs, shownLog, handleSelectLog],
  );

  useHotkeys("shift+j", () => jumpToError("next"), [jumpToError]);
  useHotkeys("shift+k", () => jumpToError("prev"), [jumpToError]);
  useHotkeys("e", () => jumpToError("next"), [jumpToError]);

  const normalizedSelectedRange = useMemo(() => {
    if (!selectedRange) return null;
    const startIndex = Math.min(
      selectedRange.startIndex,
      selectedRange.endIndex,
    );
    const endIndex = Math.max(selectedRange.startIndex, selectedRange.endIndex);
    return { startIndex, endIndex };
  }, [selectedRange]);

  const selectedLogsForCopy = useMemo(() => {
    if (!normalizedSelectedRange) return [];
    return interleavedLogs.slice(
      normalizedSelectedRange.startIndex,
      normalizedSelectedRange.endIndex + 1,
    );
  }, [interleavedLogs, normalizedSelectedRange]);

  // Use the sophisticated clipboard hook
  useLogsClipboard(interleavedLogs, outerRef, selectedLogsForCopy);

  const hasFilters =
    !!logs && !!filteredLogs && filteredLogs.length !== logs.length;

  const onScroll = useCallback(
    ({ scrollOffset }: ListOnScrollProps) => {
      if (scrollOffset === 0 && !shownLog) {
        setPaused(false);
      } else if (!paused) {
        setPaused(true);
      }
    },
    [paused, setPaused, shownLog],
  );

  return (
    <Sheet
      className="h-full w-full overflow-hidden"
      padding={false}
      ref={sheetRef}
    >
      <PanelGroup
        direction="horizontal"
        className="flex h-full w-full flex-auto overflow-hidden"
        autoSaveId="logs-content"
      >
        <Panel
          id="log-list-panel"
          order={0}
          className={cn(
            "flex shrink flex-col",
            "max-w-full",
            shownLog ? "min-w-[16rem]" : "min-w-[20rem]",
          )}
          defaultSize={100}
          minSize={10}
        >
          {interleavedLogs !== undefined && heightOfListContainer !== 0 && (
            <WindowedLogList
              {...{
                onScroll,
                interleavedLogs,
                setClearedLogs,
                clearedLogs,
                setShownLog: handleSelectLog,
                setSelectedRange,
                selectedRange,
                hasFilters,
                paused,
                setManuallyPaused,
                hitBoundary,
                shownLog,
                listRef,
                outerRef,
                filter,
              }}
            />
          )}
        </Panel>
        {shownLog && logs && (
          <>
            <ResizeHandle collapsed={false} direction="left" />
            <Panel
              id="log-drilldown-panel"
              order={1}
              defaultSize={10}
              minSize={10}
              className="flex min-w-[24rem] flex-col"
            >
              <LogDrilldown
                requestId={
                  shownLog.kind === "ExecutionLog"
                    ? shownLog.executionLog.requestId
                    : undefined
                }
                shownInterleavedLogs={interleavedLogs}
                allUdfLogs={
                  shownLog.kind === "ExecutionLog"
                    ? [...logs, ...(pausedLogs ?? [])].filter(
                        (log) =>
                          log.requestId === shownLog.executionLog.requestId,
                      )
                    : []
                }
                onClose={() => setShownLog(undefined)}
                selectedLog={shownLog}
                onFilterByRequestId={(requestId) => {
                  setFilter?.(requestId);
                }}
                onSelectLog={handleSelectLog}
                onHitBoundary={setHitBoundary}
                logListContainerRef={outerRef}
              />
            </Panel>
          </>
        )}
      </PanelGroup>
    </Sheet>
  );
}

function WindowedLogList({
  interleavedLogs,
  setClearedLogs,
  clearedLogs,
  onScroll,
  setShownLog,
  setSelectedRange,
  selectedRange,
  hasFilters,
  paused,
  setManuallyPaused,
  shownLog,
  hitBoundary,
  listRef,
  outerRef,
  filter,
}: {
  interleavedLogs: InterleavedLog[];
  setClearedLogs: (clearedLogs: number[]) => void;
  clearedLogs: number[];
  onScroll: (e: ListOnScrollProps) => void;
  setShownLog(shown: InterleavedLog | undefined): void;
  setSelectedRange: React.Dispatch<
    React.SetStateAction<{ startIndex: number; endIndex: number } | null>
  >;
  selectedRange: { startIndex: number; endIndex: number } | null;
  hasFilters: boolean;
  paused: boolean;
  setManuallyPaused(paused: boolean): void;
  shownLog?: InterleavedLog;
  hitBoundary: "top" | "bottom" | null;
  listRef: React.RefObject<FixedSizeList>;
  outerRef: React.RefObject<HTMLDivElement>;
  filter?: string;
}) {
  return (
    <div className="scrollbar flex h-full min-w-0 flex-col overflow-x-auto overflow-y-hidden">
      <div className="flex h-full min-w-fit flex-col">
        <LogListHeader
          hasLogOpen={shownLog !== undefined}
          paused={paused}
          setManuallyPaused={setManuallyPaused}
          listRef={listRef}
          outerRef={outerRef}
        />
        {interleavedLogs.length === 0 ? (
          <div className="mt-2 ml-2 animate-fadeInFromLoading text-sm text-content-secondary">
            {hasFilters && (
              <p className="mb-2 flex items-center gap-1">
                No logs match your filters{" "}
                <Tooltip
                  tip="The logs page is a realtime stream of events in this deployment. To store a longer history of logs, you may
configure a log stream."
                >
                  <InfoCircledIcon />
                </Tooltip>
              </p>
            )}
            <p className="animate-blink">Waiting for new logs...</p>
          </div>
        ) : (
          <div className="grow rounded-b-lg">
            <InfiniteScrollList
              className="scrollbar bg-background-secondary"
              style={{
                overflowX: "hidden",
              }}
              overscanCount={25}
              onScroll={onScroll}
              outerRef={outerRef}
              listRef={listRef}
              itemKey={(index) => {
                const log = interleavedLogs[index];
                return getLogKey(log);
              }}
              items={interleavedLogs}
              totalNumItems={interleavedLogs.length}
              itemSize={ITEM_SIZE}
              itemData={{
                interleavedLogs,
                setClearedLogs,
                clearedLogs,
                setShownLog,
                setSelectedRange,
                selectedRange,
                selectedLog: shownLog,
                hitBoundary,
                filter,
              }}
              RowOrLoading={LogListRow}
            />
          </div>
        )}
      </div>
    </div>
  );
}

export function LogsMenuButton({ open }: { open: boolean }) {
  return (
    <Button
      inline
      focused={open}
      variant="neutral"
      size="sm"
      className="-ml-2.5"
    >
      <h3 className="flex items-center gap-2 font-mono">
        <HamburgerMenuIcon className="mt-0.5" />
        Logs
      </h3>
    </Button>
  );
}

type LogItemProps = {
  data: {
    interleavedLogs: InterleavedLog[];
    setClearedLogs: (clearedLogs: number[]) => void;
    setShownLog(shown: InterleavedLog | undefined): void;
    setSelectedRange: React.Dispatch<
      React.SetStateAction<{ startIndex: number; endIndex: number } | null>
    >;
    selectedRange: { startIndex: number; endIndex: number } | null;
    clearedLogs: number[];
    selectedLog?: InterleavedLog;
    hitBoundary?: "top" | "bottom" | null;
    filter?: string;
  };
  index: number;
  style: any;
};

const LogListRow = memo(LogListRowImpl, areEqual);

function LogListRowImpl({ data, index, style }: LogItemProps) {
  const {
    setClearedLogs,
    clearedLogs,
    interleavedLogs,
    setShownLog,
    setSelectedRange,
    selectedRange,
    selectedLog,
    hitBoundary,
    filter,
  } = data;
  const log = interleavedLogs[index];

  const isFocused = selectedLog
    ? getLogKey(log) === getLogKey(selectedLog)
    : false;

  const logKey = getLogKey(log);
  const normalizedRange =
    selectedRange !== null
      ? {
          startIndex: Math.min(
            selectedRange.startIndex,
            selectedRange.endIndex,
          ),
          endIndex: Math.max(selectedRange.startIndex, selectedRange.endIndex),
        }
      : null;

  const isSelected =
    normalizedRange !== null &&
    index >= normalizedRange.startIndex &&
    index <= normalizedRange.endIndex;
  const hasRowSelection =
    selectedRange !== null &&
    selectedRange.startIndex !== selectedRange.endIndex;

  const handleRowClick = (e: React.MouseEvent) => {
    if (e.shiftKey && selectedRange) {
      setSelectedRange({
        startIndex: selectedRange.startIndex,
        endIndex: index,
      });
    } else {
      setSelectedRange({ startIndex: index, endIndex: index });
    }
    setShownLog(log);
  };

  let item: React.ReactNode = null;

  switch (log.kind) {
    case "ClearedLogs":
      item = (
        <ClearedLogsButton
          focused={isFocused}
          selected={isSelected}
          hitBoundary={hitBoundary}
          onClick={(e) => {
            handleRowClick(e);
            setClearedLogs(clearedLogs.slice(0, clearedLogs.length - 1));
            setShownLog(undefined);
          }}
          onFocus={() => setShownLog(log)}
          logKey={logKey}
        />
      );
      break;
    case "DeploymentEvent":
      item = (
        <DeploymentEventListItem
          event={log.deploymentEvent}
          focused={isFocused}
          selected={isSelected}
          hitBoundary={hitBoundary}
          setShownLog={() => setShownLog(log)}
          onClick={handleRowClick}
          logKey={logKey}
        />
      );
      break;
    default:
      item = (
        <LogListItem
          log={log.executionLog}
          setShownLog={() => setShownLog(log)}
          focused={isFocused}
          selected={isSelected}
          hitBoundary={hitBoundary}
          logKey={logKey}
          highlight={filter}
          onClick={handleRowClick}
          hasRowSelection={hasRowSelection}
        />
      );
      break;
  }

  return (
    <div
      style={{
        ...style,
        overflowAnchor: index === interleavedLogs.length - 1 ? "auto" : "none",
      }}
      className="overflow-hidden"
    >
      {item}
    </div>
  );
}

const CLEARED_LOGS_BUTTON_HEIGHT = 24;

function ClearedLogsButton({
  focused,
  selected,
  hitBoundary,
  onClick,
  onFocus,
  logKey,
}: {
  focused: boolean;
  selected: boolean;
  hitBoundary?: "top" | "bottom" | null;
  onClick: (e: React.MouseEvent) => void;
  onFocus: () => void;
  logKey?: string;
}) {
  const handleClick = () => {
    onFocus();
  };

  // Only show boundary animation on the focused item
  const showBoundary = focused && hitBoundary;

  return (
    <div
      style={{ height: CLEARED_LOGS_BUTTON_HEIGHT }}
      className={cn(
        showBoundary === "top" && "animate-[bounceTop_0.375s_ease-out]",
        showBoundary === "bottom" && "animate-[bounceBottom_0.375s_ease-out]",
      )}
    >
      <Button
        data-log-key={logKey}
        icon={<ArrowDownIcon />}
        inline
        size="xs"
        className={cn(
          "w-full rounded-none pl-2",
          selected && "bg-background-tertiary/50",
        )}
        style={{ height: ITEM_SIZE }}
        onClick={(e) => {
          handleClick();
          onClick(e);
        }}
        tabIndex={0}
      >
        Show previous logs
      </Button>
    </div>
  );
}

function LogListHeader({
  hasLogOpen,
  paused,
  setManuallyPaused,
  listRef,
  outerRef,
}: {
  hasLogOpen: boolean;
  paused: boolean;
  setManuallyPaused(paused: boolean): void;
  listRef: React.RefObject<FixedSizeList>;
  outerRef: React.RefObject<HTMLDivElement>;
}) {
  return (
    <div className="flex w-full items-center gap-4 border-b p-1 pl-2.5 text-xs text-content-secondary">
      <TimestampColumn />
      <div className="flex min-w-8 items-center gap-1 text-center">
        ID
        <Tooltip tip="The first few characters of the ID of the request that triggered this log. Selecting a log in this list will show all logs for that request.">
          <QuestionMarkCircledIcon />
        </Tooltip>
      </div>
      <StatusColumn />
      <FunctionColumn />

      <div className={cn("sticky right-1", hasLogOpen ? "shadow-lg" : "")}>
        <Button
          size="xs"
          className="text-xs"
          icon={
            paused ? (
              <PlayCircleIcon className="size-4" />
            ) : (
              <PauseCircleIcon className="size-4" />
            )
          }
          onClick={() => {
            if (paused) {
              listRef.current?.scrollToItem(0);
            }
            setManuallyPaused(!paused);
          }}
          tip={
            paused
              ? "Resume to show live log updates."
              : "Pause to prevent live log updates."
          }
        >
          {paused
            ? `Go Live${(outerRef.current?.scrollTop || 0) > 0 ? " ↑" : ""}`
            : "Pause"}
        </Button>
      </div>
    </div>
  );
}

function TimestampColumn() {
  return (
    <div className="flex min-w-[9.25rem] items-center gap-1">
      Timestamp
      <Tooltip tip="Logs are sorted by timestamp, with the most recent logs appearing first.">
        <CaretDownIcon />
      </Tooltip>
    </div>
  );
}
function FunctionColumn() {
  return <div className="flex min-w-60 grow items-center gap-1">Function</div>;
}

function StatusColumn() {
  return <div className="min-w-[7rem]">Status</div>;
}
