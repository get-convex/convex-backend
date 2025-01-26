import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useDebounce, usePrevious } from "react-use";
import isEqual from "lodash/isEqual";
import { dismissToast, toast } from "../../../lib/utils";
import { LogList } from "./LogList";
import { functionsForSelectedNents, LogToolbar } from "./LogToolbar";
import { filterLogs } from "../lib/filterLogs";
import { NENT_APP_PLACEHOLDER, Nent } from "../../../lib/useNents";
import {
  itemIdentifier,
  useModuleFunctions,
} from "../../../lib/functions/FunctionsProvider";
import { functionIdentifierValue } from "../../../lib/functions/generateFileTree";
import { MAX_LOGS, UdfLog, useLogs } from "../../../lib/useLogs";
import { useDeploymentAuditLogs } from "../../../lib/useDeploymentAuditLog";
import { TextInput } from "../../../elements/TextInput";
import { Button } from "../../../elements/Button";

export function Logs({
  nents: allNents,
  selectedNent,
}: {
  nents: Nent[];
  selectedNent: Nent | null;
}) {
  const nents = allNents.filter((nent) => nent.name !== null);
  const logsConnectivityCallbacks = useRef({
    onReconnected: () => {
      dismissToast("logStreamError");
      toast("info", "Reconnected to log stream.", "logStreamReconnected");
    },
    onDisconnected: () => {
      dismissToast("logStreamReconnected");
      toast(
        "error",
        "Disconnected from log stream. Will attempt to reconnect automatically.",
        "logStreamError",
        false,
      );
    },
  });

  // Manage state for filter text.
  const [filter, setFilter] = useState("");

  // Manage state for current log levels.
  // TODO: Use query params to track this state.
  const [levels, setLevels] = useState([
    "success",
    "failure",
    "DEBUG",
    "INFO",
    "WARN",
    "ERROR",
  ]);

  const [selectedNents, setSelectedNents] = useState<string[]>([
    selectedNent ? selectedNent.path : NENT_APP_PLACEHOLDER,
  ]);

  const moduleFunctions = useModuleFunctions();
  const functions = useMemo(
    () => [
      ...moduleFunctions.map((value) => itemIdentifier(value)),
      functionIdentifierValue("_other"),
    ],
    [moduleFunctions],
  );

  const [selectedFunctions, setSelectedFunctions] = useState<string[]>(
    functionsForSelectedNents(selectedNents, functions),
  );

  const [logs, setLogs] = useState<UdfLog[]>([]);
  const [filteredLogs, setFilteredLogs] = useState<UdfLog[]>([]);

  const filters = useMemo(
    () => ({
      logTypes: levels,
      functions,
      selectedFunctions,
      filter,
    }),
    [filter, functions, levels, selectedFunctions],
  );
  const previousFilters = usePrevious(filters);

  const [clearedLogs, setClearedLogs] = useState<number[]>([]);

  const [fromTimestamp, setFromTimestamp] = useState<number>();
  const deploymentAuditLogs = useDeploymentAuditLogs(fromTimestamp);

  const receiveLogs = useCallback(
    (entries: UdfLog[]) => {
      setLogs((prev) =>
        [...prev, ...entries].slice(
          Math.max(prev.length + entries.length - MAX_LOGS, 0),
          prev.length + entries.length,
        ),
      );
      setFilteredLogs((prev) =>
        [...prev, ...(filterLogs(filters, entries) || [])].slice(
          Math.max(prev.length + entries.length - MAX_LOGS, 0),
          prev.length + entries.length,
        ),
      );
    },
    [filters],
  );

  const [manuallyPaused, setManuallyPaused] = useState(false);
  const [paused, setPaused] = useState<number>(0);
  const onPause = (p: boolean) => {
    const now = new Date().getTime();
    setPaused(p ? now : 0);
  };
  useLogs(
    logsConnectivityCallbacks.current,
    receiveLogs,
    paused > 0 || manuallyPaused,
  );

  useEffect(() => {
    if (isEqual(filters, previousFilters)) {
      return;
    }
    const newFilteredLogs = filterLogs(filters, logs) || [];
    setFilteredLogs(newFilteredLogs);
  }, [filters, previousFilters, logs]);

  const [innerFilter, setInnerFilter] = useState(filter);
  useDebounce(
    () => {
      setFilter(innerFilter);
    },
    200,
    [innerFilter],
  );

  // Note: fromTimestamp used to be a `useMemo` result, but it was causing a bug
  // where fromTimestamp would keep changing and causing the query to be refetched
  // every time the first log entry changed
  // (which shouldn't happen, but I haven't debugged why that does happen yet).
  useEffect(() => {
    if (logs && logs[0] && fromTimestamp === undefined) {
      setFromTimestamp(logs[0].timestamp);
    }
  }, [logs, fromTimestamp]);

  const latestLog = logs?.at(-1);
  const latestAuditLog = deploymentAuditLogs?.at(-1);
  const latestTimestamp =
    (latestLog?.timestamp ?? 0) > (latestAuditLog?._creationTime ?? 0)
      ? latestLog?.timestamp
      : latestAuditLog?._creationTime;

  return (
    <div className="flex h-full w-full min-w-[48rem] flex-col gap-2 p-6 py-4">
      <div className="flex flex-col gap-4">
        <LogToolbar
          firstItem={<LogsHeader />}
          selectedLevels={Array.from(levels)}
          selectedFunctions={selectedFunctions}
          setSelectedFunctions={setSelectedFunctions}
          functions={functions}
          setSelectedLevels={setLevels}
          nents={
            nents.length >= 1
              ? [NENT_APP_PLACEHOLDER, ...nents.map((nent) => nent.path)]
              : undefined
          }
          selectedNents={selectedNents}
          setSelectedNents={setSelectedNents}
        />
        <div className="mb-2 flex w-full gap-2">
          <TextInput
            id="Search logs"
            placeholder="Filter logs..."
            value={innerFilter}
            onChange={(e) => setInnerFilter(e.target.value)}
            type="search"
          />
          <Button
            size="sm"
            variant="neutral"
            tip="Clear the currently visible logs to declutter this page."
            tipSide="left"
            disabled={
              latestTimestamp === undefined ||
              !logs ||
              (clearedLogs.length
                ? logs.filter(
                    (log) =>
                      log.timestamp > clearedLogs[clearedLogs.length - 1],
                  )
                : logs
              ).length === 0
            }
            onClick={() => {
              setClearedLogs([...clearedLogs, latestTimestamp!]);
            }}
          >
            Clear Logs
          </Button>
        </div>
      </div>
      <LogList
        nents={nents}
        logs={logs}
        filteredLogs={filteredLogs}
        deploymentAuditLogs={deploymentAuditLogs}
        filter={filter}
        clearedLogs={clearedLogs}
        setClearedLogs={setClearedLogs}
        paused={paused > 0 || manuallyPaused}
        setPaused={onPause}
        setManuallyPaused={(p) => {
          onPause(p);
          setManuallyPaused(p);
        }}
      />
    </div>
  );
}

function LogsHeader() {
  return (
    <div className="mr-2 flex grow items-center justify-between gap-2">
      <div className="flex items-center gap-2">
        <h3>Logs</h3>
      </div>
    </div>
  );
}
