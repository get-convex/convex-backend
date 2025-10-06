import {
  ArrowDownIcon,
  CaretDownIcon,
  CaretUpIcon,
  HamburgerMenuIcon,
  InfoCircledIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import {
  Fragment,
  memo,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { FixedSizeList, ListOnScrollProps, areEqual } from "react-window";
import { useDebounce, useMeasure } from "react-use";
import { Transition, Dialog } from "@headlessui/react";
import isEqual from "lodash/isEqual";
import { PauseCircleIcon, PlayCircleIcon } from "@heroicons/react/24/outline";
import { DeploymentEventListItem } from "@common/features/logs/components/DeploymentEventListItem";
import {
  ITEM_SIZE,
  LogListItem,
} from "@common/features/logs/components/LogListItem";
import { LogToolbar } from "@common/features/logs/components/LogToolbar";
import { filterLogs } from "@common/features/logs/lib/filterLogs";
import { UdfLog } from "@common/lib/useLogs";
import {
  InterleavedLog,
  interleaveLogs,
  getTimestamp,
} from "@common/features/logs/lib/interleaveLogs";
import { DeploymentAuditLogEvent } from "@common/lib/useDeploymentAuditLog";
import { NENT_APP_PLACEHOLDER, Nent } from "@common/lib/useNents";
import { Sheet } from "@ui/Sheet";
import { Tooltip } from "@ui/Tooltip";
import { InfiniteScrollList } from "@common/elements/InfiniteScrollList";
import { Button } from "@ui/Button";
import { ClosePanelButton } from "@ui/ClosePanelButton";
import { CopyTextButton } from "@common/elements/CopyTextButton";
import { TextInput } from "@ui/TextInput";
import { MultiSelectValue } from "@ui/MultiSelectCombobox";
import { LogListResources } from "@common/features/logs/components/LogListResources";
import { shallowNavigate } from "@common/lib/useTableMetadata";
import { useRouter } from "next/router";
import { Panel, PanelGroup } from "react-resizable-panels";
import { cn } from "@ui/cn";
import { ResizeHandle } from "@common/layouts/SidebarDetailLayout";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { LogDrilldown } from "./LogDrilldown";

export type LogListProps = {
  logs?: UdfLog[];
  filteredLogs?: UdfLog[];
  deploymentAuditLogs?: DeploymentAuditLogEvent[];
  filter: string;
  setFilter?: (filter: string) => void;
  clearedLogs: number[];
  setClearedLogs: (clearedLogs: number[]) => void;
  nents: Nent[];
  paused: boolean;
  setPaused: (paused: boolean) => void;
  setManuallyPaused: (paused: boolean) => void;
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
  filteredLogs,
  deploymentAuditLogs,
  clearedLogs,
  setClearedLogs,
  nents,
  paused,
  setPaused,
  setManuallyPaused,
  setFilter,
}: LogListProps) {
  const router = useRouter();

  const interleavedLogs = interleaveLogs(
    filteredLogs ?? [],
    deploymentAuditLogs ?? [],
    clearedLogs,
  ).reverse();

  const [sheetRef, { height: heightOfListContainer }] =
    useMeasure<HTMLDivElement>();

  // Local state for hit boundary with automatic timeout reset
  const { hitBoundary, setHitBoundary } = useHitBoundary();

  // Derive shownLog from URL query parameters - now supports all interleaved log types
  const shownLog = useMemo(() => {
    const logTs = router.query.logTs as string | undefined;

    if (logTs && interleavedLogs) {
      // Find the log with this timestamp in interleaved logs
      const matchingLog = interleavedLogs.find(
        (log) => getTimestamp(log) === Number(logTs),
      );
      return matchingLog;
    }

    return undefined;
  }, [router.query.logTs, interleavedLogs]);

  // Update URL when log selection changes
  const setShownLog = useCallback(
    (log: InterleavedLog | UdfLog | undefined) => {
      if (!log) {
        void shallowNavigate(router, {
          ...router.query,
          logTs: undefined,
        });
        return;
      }
      const timestamp = "timestamp" in log ? log.timestamp : getTimestamp(log);
      void shallowNavigate(router, {
        ...router.query,
        logTs: timestamp.toString(),
      });
    },
    [router],
  );

  const selectLogByTimestamp = useCallback(
    (timestamp: number) => {
      void shallowNavigate(router, {
        ...router.query,
        logTs: timestamp.toString(),
      });
    },
    [router],
  );

  const hasFilters =
    !!logs && !!filteredLogs && filteredLogs.length !== logs.length;

  const onScroll = useCallback(
    ({ scrollOffset }: ListOnScrollProps) => {
      if (scrollOffset === 0) {
        setPaused(false);
      } else {
        !paused && setPaused(true);
      }
    },
    [paused, setPaused],
  );

  const { newLogsPageSidepanel } = useContext(DeploymentInfoContext);

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
          className={cn(
            "flex shrink flex-col",
            "max-w-full",
            shownLog ? "min-w-[16rem]" : "min-w-[20rem]",
          )}
          defaultSize={shownLog ? 60 : 100}
          minSize={10}
        >
          {interleavedLogs !== undefined && heightOfListContainer !== 0 && (
            <WindowedLogList
              {...{
                onScroll,
                interleavedLogs,
                setClearedLogs,
                clearedLogs,
                setShownLog,
                hasFilters,
                paused,
                setManuallyPaused,
                hitBoundary,
                shownLog,
                newLogsPageSidepanel,
              }}
            />
          )}
        </Panel>
        {shownLog &&
          logs &&
          (newLogsPageSidepanel ? (
            <>
              <ResizeHandle collapsed={false} direction="left" />
              <Panel
                defaultSize={0}
                minSize={10}
                className="flex min-w-[24rem] flex-col"
              >
                <LogDrilldown
                  requestId={
                    shownLog.kind === "ExecutionLog"
                      ? shownLog.executionLog.requestId
                      : undefined
                  }
                  logs={interleavedLogs ?? []}
                  onClose={() => setShownLog(undefined)}
                  selectedLogTimestamp={getTimestamp(shownLog)}
                  onFilterByRequestId={(requestId) => {
                    setFilter?.(requestId);
                  }}
                  onSelectLog={selectLogByTimestamp}
                  onHitBoundary={setHitBoundary}
                />
              </Panel>
            </>
          ) : (
            shownLog.kind === "ExecutionLog" && (
              <RequestIdLogs
                requestId={shownLog.executionLog}
                logs={logs.filter(
                  (log) => log.requestId === shownLog.executionLog.requestId,
                )}
                onClose={() => setShownLog(undefined)}
                nents={nents}
              />
            )
          ))}
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
  hasFilters,
  paused,
  setManuallyPaused,
  shownLog,
  hitBoundary,
  newLogsPageSidepanel,
}: {
  interleavedLogs: InterleavedLog[];
  setClearedLogs: (clearedLogs: number[]) => void;
  clearedLogs: number[];
  onScroll: (e: ListOnScrollProps) => void;
  setShownLog(shown: InterleavedLog | UdfLog | undefined): void;
  hasFilters: boolean;
  paused: boolean;
  setManuallyPaused(paused: boolean): void;
  shownLog?: InterleavedLog;
  hitBoundary: "top" | "bottom" | null;
  newLogsPageSidepanel?: boolean;
}) {
  const listRef = useRef<FixedSizeList>(null);
  const outerRef = useRef<HTMLDivElement>(null);

  return (
    <div className="scrollbar flex h-full min-w-0 flex-col overflow-x-auto overflow-y-hidden">
      <div className="flex h-full min-w-fit flex-col">
        <LogListHeader
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
                switch (log.kind) {
                  case "ExecutionLog":
                    return log.executionLog.id;
                  case "DeploymentEvent":
                    return log.deploymentEvent._id;
                  default:
                    return "clearedLogs";
                }
              }}
              items={interleavedLogs}
              totalNumItems={interleavedLogs.length}
              itemSize={ITEM_SIZE}
              itemData={{
                interleavedLogs,
                setClearedLogs,
                clearedLogs,
                setShownLog,
                selectedLogTimestamp: shownLog
                  ? getTimestamp(shownLog)
                  : undefined,
                hitBoundary,
                newLogsPageSidepanel,
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
    setShownLog(shown: InterleavedLog | UdfLog | undefined): void;
    clearedLogs: number[];
    selectedLogTimestamp?: number;
    hitBoundary?: "top" | "bottom" | null;
    newLogsPageSidepanel?: boolean;
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
    selectedLogTimestamp,
    hitBoundary,
    newLogsPageSidepanel,
  } = data;
  const log = interleavedLogs[index];

  let item: React.ReactNode = null;

  switch (log.kind) {
    case "ClearedLogs":
      item = (
        <ClearedLogsButton
          focused={getTimestamp(log) === selectedLogTimestamp}
          hitBoundary={hitBoundary}
          onClick={() => {
            setClearedLogs(clearedLogs.slice(0, clearedLogs.length - 1));
          }}
          onFocus={() => newLogsPageSidepanel && setShownLog(log)}
          newLogsPageSidepanel={newLogsPageSidepanel}
        />
      );
      break;
    case "DeploymentEvent":
      item = (
        <DeploymentEventListItem
          event={log.deploymentEvent}
          focused={getTimestamp(log) === selectedLogTimestamp}
          hitBoundary={hitBoundary}
          setShownLog={() => setShownLog(log)}
          onCloseDialog={() => setShownLog(undefined)}
          newLogsPageSidepanel={newLogsPageSidepanel}
        />
      );
      break;
    default:
      item = (
        <LogListItem
          log={log.executionLog}
          setShownLog={setShownLog}
          focused={getTimestamp(log) === selectedLogTimestamp}
          hitBoundary={hitBoundary}
          newLogsPageSidepanel={newLogsPageSidepanel}
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
  hitBoundary,
  onClick,
  onFocus,
  newLogsPageSidepanel,
}: {
  focused: boolean;
  hitBoundary?: "top" | "bottom" | null;
  onClick: () => void;
  onFocus: () => void;
  newLogsPageSidepanel?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const prevFocusedRef = useRef(focused);

  // Focus the button when focused prop changes to true
  useEffect(() => {
    if (focused) {
      buttonRef.current?.focus();
    }
  }, [focused]);

  // Scroll into view when transitioning to focused (only in side panel mode)
  useEffect(() => {
    if (
      focused &&
      !prevFocusedRef.current &&
      ref.current &&
      newLogsPageSidepanel
    ) {
      ref.current.scrollIntoView({
        block: "center",
        inline: "nearest",
      });
    }
    prevFocusedRef.current = focused;
  }, [focused, ref, newLogsPageSidepanel]);

  const handleClick = () => {
    onClick();
    onFocus();
  };

  // Only show boundary animation on the focused item
  const showBoundary = focused && hitBoundary;

  return (
    <div
      ref={ref}
      style={{ height: CLEARED_LOGS_BUTTON_HEIGHT }}
      className={cn(
        showBoundary === "top" && "animate-[bounceTop_0.375s_ease-out]",
        showBoundary === "bottom" && "animate-[bounceBottom_0.375s_ease-out]",
      )}
    >
      <Button
        ref={buttonRef}
        icon={<ArrowDownIcon />}
        inline
        size="xs"
        className="w-full rounded-none pl-2"
        style={{ height: ITEM_SIZE }}
        onClick={handleClick}
        tabIndex={0}
      >
        Show previous logs
      </Button>
    </div>
  );
}

function RequestIdLogs({
  requestId,
  logs,
  onClose,
  nents,
}: {
  requestId: { requestId: string; executionId: string };
  logs: UdfLog[];
  onClose: () => void;
  nents: Nent[];
}) {
  const [isOpen, setIsOpen] = useState(true);
  const [filter, setFilter] = useState("");

  const handleClose = () => {
    // Blur the currently focused element to prevent focus from returning to the list button
    // which would re-trigger the selection via onFocus
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
    setIsOpen(false);
  };

  const handleAfterLeave = () => {
    onClose();
  };

  const functions = Array.from(
    new Set(
      logs.flatMap((log) => {
        const logFunctions = [log.call];
        if (log.kind === "log" && log.output.subfunction !== undefined) {
          logFunctions.push(log.output.subfunction);
        }
        return logFunctions;
      }),
    ),
  );
  const [selectedFunctions, setSelectedFunctions] =
    useState<MultiSelectValue>("all");

  const [selectedLevels, setSelectedLevels] = useState<MultiSelectValue>("all");

  const filters = {
    logTypes: selectedLevels,
    functions,
    selectedFunctions,
    selectedNents: "all" as MultiSelectValue,
    filter,
  };

  const [innerFilter, setInnerFilter] = useState(filter);
  useDebounce(
    () => {
      setFilter(innerFilter);
    },
    200,
    [innerFilter],
  );

  const filteredLogs = filterLogs(filters, logs);

  return (
    <Transition.Root
      show={isOpen}
      as={Fragment}
      appear
      afterLeave={handleAfterLeave}
    >
      <Dialog
        as="div"
        className="fixed inset-0 z-40 overflow-hidden"
        onClose={handleClose}
      >
        <div className="absolute inset-0 overflow-hidden">
          <Transition.Child
            as={Fragment}
            enter="ease-in-out duration-300"
            enterFrom="opacity-0"
            enterTo="opacity-100"
            leave="ease-in-out duration-300"
            leaveFrom="opacity-100"
            leaveTo="opacity-0"
          >
            <Dialog.Overlay className="absolute inset-0" />
          </Transition.Child>

          <div className="fixed inset-y-0 right-0 flex max-w-full pl-10">
            <Transition.Child
              as={Fragment}
              enter="transform transition ease-in-out duration-300"
              enterFrom="translate-x-full"
              enterTo="translate-x-0"
              leave="transform transition ease-in-out duration-300"
              leaveFrom="translate-x-0"
              leaveTo="translate-x-full"
            >
              <div className="w-screen max-w-md sm:max-w-lg md:max-w-xl lg:max-w-3xl xl:max-w-5xl">
                <div className="flex h-full max-h-full flex-col bg-background-secondary shadow-xl dark:border">
                  {/* Header */}
                  <div className="mb-1 px-6 pt-6">
                    <div className="flex items-center justify-between gap-4">
                      <Dialog.Title as="h4" className="flex items-center gap-2">
                        Request breakdown{" "}
                        <CopyTextButton
                          className="font-mono text-xs font-semibold"
                          text={requestId.requestId}
                        />
                      </Dialog.Title>
                      <ClosePanelButton onClose={handleClose} />
                    </div>
                  </div>
                  <LogListResources logs={logs} />
                  <div className="mx-6 mt-2 flex flex-col gap-2">
                    <LogToolbar
                      firstItem={<h5 className="grow">Logs</h5>}
                      functions={functions}
                      selectedFunctions={selectedFunctions}
                      setSelectedFunctions={setSelectedFunctions}
                      selectedLevels={selectedLevels}
                      setSelectedLevels={setSelectedLevels}
                      selectedNents={[
                        ...nents.map((n) => n.path),
                        NENT_APP_PLACEHOLDER,
                      ]}
                      // Nents are not used in this view
                      setSelectedNents={() => {}}
                    />
                    <TextInput
                      id="Search logs"
                      outerClassname="w-full"
                      placeholder="Filter logs..."
                      value={innerFilter}
                      onChange={(e) => setInnerFilter(e.target.value)}
                      type="search"
                    />
                  </div>
                  {filteredLogs && filteredLogs.length > 0 ? (
                    <div className="mx-6 my-4 flex grow flex-col overflow-y-hidden rounded-sm border text-xs">
                      <RequestIdLogsHeader />
                      <div className="scrollbar flex grow flex-col divide-y overflow-y-auto font-mono">
                        {filteredLogs.map((log, idx) => (
                          <LogListItem
                            key={idx}
                            log={log}
                            focused={isEqual(log, requestId)}
                          />
                        ))}
                      </div>
                    </div>
                  ) : (
                    <div className="mx-6 mt-4 text-sm text-content-secondary">
                      No logs match your filters.
                    </div>
                  )}
                </div>
              </div>
            </Transition.Child>
          </div>
        </div>
      </Dialog>
    </Transition.Root>
  );
}

function LogListHeader({
  paused,
  setManuallyPaused,
  listRef,
  outerRef,
}: {
  paused: boolean;
  setManuallyPaused(paused: boolean): void;
  listRef: React.RefObject<FixedSizeList>;
  outerRef: React.RefObject<HTMLDivElement>;
}) {
  return (
    <div className="flex items-center gap-4 border-b p-1 pl-2.5 text-xs text-content-secondary">
      <TimestampColumn />
      <div className="flex min-w-8 items-center gap-1 text-center">
        ID
        <Tooltip tip="The first few characters of the ID of the request that triggered this log. Selecting a log in this list will show all logs for that request.">
          <QuestionMarkCircledIcon />
        </Tooltip>
      </div>
      <StatusColumn />
      <FunctionColumn />

      <div className="ml-auto">
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

function RequestIdLogsHeader() {
  return (
    <div className="flex items-center gap-4 border-b py-2 pl-2 text-xs text-content-secondary">
      <div className="flex min-w-[9.25rem] items-center gap-1">
        Timestamp
        <Tooltip tip="Logs are sorted by timestamp, with the oldest logs appearing first.">
          <CaretUpIcon />
        </Tooltip>
      </div>
      {/* Not showing any other columns except timestamp for now because of the varied content shown in LogListItem in the RequestIdLogsView */}
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
  return <div className="flex min-w-60 items-center gap-1">Function</div>;
}

function StatusColumn() {
  return <div className="min-w-[7rem]">Status</div>;
}
