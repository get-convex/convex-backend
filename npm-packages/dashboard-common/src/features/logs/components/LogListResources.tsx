import { formatBytes } from "@common/lib/format";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";
import type { UdfLog } from "@common/lib/useLogs";
import { Disclosure } from "@headlessui/react";
import {
  ChevronDownIcon,
  ChevronUpIcon,
  PieChartIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import { Tooltip } from "@ui/Tooltip";
import { UsageStats } from "system-udfs/convex/_system/frontend/common";

type RequestUsageStats = UsageStats & {
  actionsRuntimeMs: number;
  actionComputeMbMs: number;
  returnBytes?: number;
};

export function LogListResources({ logs }: { logs: UdfLog[] }) {
  // Aggregate usage stats across all logs within this request, including total action runtime.
  const usageStats = (() => {
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

    return logs.reduce((accumulated, log) => {
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
  })();

  const [shouldBeOpen, setShouldBeOpen] = useGlobalLocalStorage(
    "shouldLogResourcesUsedBeOpen",
    true,
  );

  return (
    <Disclosure defaultOpen={shouldBeOpen}>
      {({ open }) => (
        <div className="mx-6 my-2 rounded-md border">
          <Disclosure.Button
            className="flex w-full items-center gap-1 p-2"
            onClick={() => setShouldBeOpen(!shouldBeOpen)}
          >
            <PieChartIcon className="relative -left-0.5 size-4" />
            <h5>Resources used</h5>
            {open ? (
              <ChevronUpIcon className="size-4" />
            ) : (
              <ChevronDownIcon className="size-4" />
            )}
          </Disclosure.Button>
          <Disclosure.Panel className="animate-fadeInFromLoading p-2 pt-0 text-xs">
            <ul className="divide-y">
              <li className="flex items-center justify-between py-2">
                <span className="text-content-secondary">Action Compute</span>
                <span className="text-content-primary">
                  <strong>
                    {Number(
                      usageStats.actionComputeMbMs / (1024 * 3_600_000),
                    ).toFixed(7)}{" "}
                    GB-hr
                  </strong>{" "}
                  ({usageStats.actionMemoryUsedMb ?? 0} MB for{" "}
                  {Number(usageStats.actionsRuntimeMs / 1000).toFixed(2)}
                  s)
                </span>
              </li>
              <li className="flex items-center justify-between py-2">
                <span className="text-content-secondary">
                  Database Bandwidth
                </span>
                <span className="text-content-primary">
                  Accessed{" "}
                  <strong>
                    {usageStats.databaseReadDocuments.toLocaleString()}{" "}
                    {usageStats.databaseReadDocuments === 1
                      ? "document"
                      : "documents"}
                  </strong>
                  , <strong>{formatBytes(usageStats.databaseReadBytes)}</strong>{" "}
                  read,{" "}
                  <strong>{formatBytes(usageStats.databaseWriteBytes)}</strong>{" "}
                  written
                </span>
              </li>
              <li className="flex items-center justify-between py-2">
                <span className="text-content-secondary">File Bandwidth</span>
                <span className="text-content-primary">
                  <strong>{formatBytes(usageStats.storageReadBytes)}</strong>{" "}
                  read,{" "}
                  <strong>{formatBytes(usageStats.storageWriteBytes)}</strong>{" "}
                  written
                </span>
              </li>
              <li className="flex items-center justify-between py-2">
                <span className="text-content-secondary">Vector Bandwidth</span>
                <span className="text-content-primary">
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
                <li className="flex items-center justify-between py-2">
                  <span className="flex items-center gap-1 text-center text-content-secondary">
                    Return Value Size
                    <Tooltip tip="Bandwidth from sending the return value of a function call to the user does not incur costs.">
                      <QuestionMarkCircledIcon />
                    </Tooltip>
                  </span>
                  <span className="text-content-primary">
                    <strong>{formatBytes(usageStats.returnBytes)}</strong>{" "}
                    returned
                  </span>
                </li>
              )}
            </ul>
            {logs.filter((log) => log.kind === "outcome").length > 1 && (
              <div className="mt-3 text-content-secondary">
                Total resources used across{" "}
                {logs.filter((l) => l.kind === "outcome").length} executions in
                this request.
              </div>
            )}
          </Disclosure.Panel>
        </div>
      )}
    </Disclosure>
  );
}
