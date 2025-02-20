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
  useEffect,
  useRef,
  useState,
} from "react";
import { FixedSizeList, ListOnScrollProps, areEqual } from "react-window";
import { useDebounce, useMeasure, usePrevious } from "react-use";
import { Transition, Dialog } from "@headlessui/react";
import isEqual from "lodash/isEqual";
import difference from "lodash/difference";
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
} from "@common/features/logs/lib/interleaveLogs";
import { DeploymentAuditLogEvent } from "@common/lib/useDeploymentAuditLog";
import { NENT_APP_PLACEHOLDER, Nent } from "@common/lib/useNents";
import { Sheet } from "@common/elements/Sheet";
import { Tooltip } from "@common/elements/Tooltip";
import { InfiniteScrollList } from "@common/elements/InfiniteScrollList";
import { Button } from "@common/elements/Button";
import { ClosePanelButton } from "@common/elements/ClosePanelButton";
import { CopyTextButton } from "@common/elements/CopyTextButton";
import { TextInput } from "@common/elements/TextInput";

export type LogListProps = {
  logs?: UdfLog[];
  filteredLogs?: UdfLog[];
  deploymentAuditLogs?: DeploymentAuditLogEvent[];
  filter: string;
  clearedLogs: number[];
  setClearedLogs: (clearedLogs: number[]) => void;
  nents: Nent[];
  paused: boolean;
  setPaused: (paused: boolean) => void;
  setManuallyPaused: (paused: boolean) => void;
};

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
}: LogListProps) {
  const interleavedLogs = interleaveLogs(
    filteredLogs ?? [],
    deploymentAuditLogs ?? [],
    clearedLogs,
  ).reverse();

  const [sheetRef, { height: heightOfListContainer }] =
    useMeasure<HTMLDivElement>();

  const [shownLog, setShownLog] = useState<UdfLog>();

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
  return (
    <div className="flex h-full w-full flex-auto flex-col gap-2 overflow-hidden">
      {shownLog && logs && (
        <RequestIdLogs
          requestId={shownLog}
          logs={logs.filter((log) => log.requestId === shownLog?.requestId)}
          onClose={() => setShownLog(undefined)}
          nents={nents}
        />
      )}
      {interleavedLogs !== undefined && (
        <Sheet className="h-full w-full" padding={false} ref={sheetRef}>
          {heightOfListContainer !== 0 && (
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
              }}
            />
          )}
        </Sheet>
      )}
    </div>
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
}: {
  interleavedLogs: InterleavedLog[];
  setClearedLogs: (clearedLogs: number[]) => void;
  clearedLogs: number[];
  onScroll: (e: ListOnScrollProps) => void;
  setShownLog(shown: UdfLog | undefined): void;
  hasFilters: boolean;
  paused: boolean;
  setManuallyPaused(paused: boolean): void;
}) {
  const listRef = useRef<FixedSizeList>(null);
  const outerRef = useRef<HTMLDivElement>(null);

  return (
    <div className="flex h-full flex-col">
      <LogListHeader
        paused={paused}
        setManuallyPaused={setManuallyPaused}
        listRef={listRef}
        outerRef={outerRef}
      />
      {interleavedLogs.length === 0 ? (
        <div className="ml-2 mt-2 animate-fadeInFromLoading text-sm text-content-secondary">
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
        <div className="grow">
          <InfiniteScrollList
            className="rounded-b bg-background-secondary scrollbar"
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
            }}
            RowOrLoading={LogListRow}
          />
        </div>
      )}
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
    setShownLog(shown: UdfLog | undefined): void;
    clearedLogs: number[];
  };
  index: number;
  style: any;
};

const LogListRow = memo(LogListRowImpl, areEqual);

function LogListRowImpl({ data, index, style }: LogItemProps) {
  const { setClearedLogs, clearedLogs, interleavedLogs, setShownLog } = data;
  const log = interleavedLogs[index];

  let item: React.ReactNode = null;

  switch (log.kind) {
    case "ClearedLogs":
      item = (
        <div style={{ height: CLEARED_LOGS_BUTTON_HEIGHT }}>
          <Button
            icon={<ArrowDownIcon />}
            inline
            size="xs"
            className="w-full rounded-none pl-2"
            style={{ height: ITEM_SIZE }}
            onClick={() => {
              setClearedLogs(clearedLogs.slice(0, clearedLogs.length - 1));
            }}
          >
            Show previous logs
          </Button>
        </div>
      );
      break;
    case "DeploymentEvent":
      item = <DeploymentEventListItem event={log.deploymentEvent} />;
      break;
    default:
      item = <LogListItem log={log.executionLog} setShownLog={setShownLog} />;
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

const CLEARED_LOGS_BUTTON_HEIGHT = 36;

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
  const [filter, setFilter] = useState("");

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
  const previousFunctions = usePrevious(functions);
  const [selectedFunctions, setSelectedFunctions] =
    useState<string[]>(functions);

  // If there are new functions in the list of logs, add them to the selected functions.
  useEffect(() => {
    if (functions.length === previousFunctions?.length) {
      return;
    }
    const newFunctions = previousFunctions
      ? difference(functions, previousFunctions)
      : [];
    setSelectedFunctions((current) => [...current, ...newFunctions]);
  }, [functions, previousFunctions]);

  const [selectedLevels, setSelectedLevels] = useState<string[]>([
    "success",
    "failure",
    "DEBUG",
    "INFO",
    "WARN",
    "ERROR",
  ]);

  const filters = {
    logTypes: selectedLevels,
    functions,
    selectedFunctions,
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
    <Transition.Root show as={Fragment} appear afterLeave={onClose}>
      <Dialog
        as="div"
        className="fixed inset-0 z-40 overflow-hidden"
        onClose={onClose}
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
                      <Dialog.Title as="h4">Request logs</Dialog.Title>
                      <ClosePanelButton onClose={onClose} />
                    </div>
                  </div>

                  <div className="mx-6 flex flex-col gap-2">
                    <LogToolbar
                      firstItem={
                        <span className="flex grow items-center gap-2 text-sm text-content-secondary">
                          Logs filtered to request:{" "}
                          <CopyTextButton
                            className="font-mono text-xs font-semibold"
                            text={requestId.requestId}
                          />
                        </span>
                      }
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
                    <div className="mx-6 my-4 flex grow flex-col overflow-y-hidden rounded border text-xs">
                      <RequestIdLogsHeader />
                      <div className="flex grow flex-col divide-y overflow-y-auto font-mono scrollbar">
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
      <div className="flex min-w-[7.5rem] items-center gap-1">
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
    <div className="flex min-w-[7.5rem] items-center gap-1">
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
