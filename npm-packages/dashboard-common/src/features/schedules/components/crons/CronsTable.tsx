import { jsonToConvex, JSONValue } from "convex/values";
import Link from "next/link";
import { useRouter } from "next/router";
import { useMemo, useState } from "react";
import { CellProps, useTable } from "react-table";
import formatDuration from "date-fns/formatDuration";
import { ChevronRightIcon, ExternalLinkIcon } from "@radix-ui/react-icons";
import {
  CronSchedule,
  CronJobLog,
  CronJobWithRuns,
} from "system-udfs/convex/_system/frontend/common";
import { useWasmCron } from "@common/features/schedules/lib/useWasmCron";
import {
  prettierSaffron,
  scheduleAsCron,
  scheduleLiteral,
} from "@common/features/schedules/lib/cronHelpers";
import { stringifyValue } from "@common/lib/stringifyValue";
import { prettier } from "@common/lib/format";
import { Tooltip } from "@ui/Tooltip";
import { useFunctionUrl } from "@common/lib/deploymentApi";
import { displayName } from "@common/lib/functions/generateFileTree";
import { LiveTimestampDistance } from "@common/elements/TimestampDistance";
import { Button } from "@ui/Button";
import { DetailPanel } from "@common/elements/DetailPanel";
import { ReadonlyCode } from "@common/elements/ReadonlyCode";
import { Sheet } from "@ui/Sheet";
import { Doc } from "system-udfs/convex/_generated/dataModel";

const COLUMN_STYLES = [
  { fontWeight: "500", flex: "2 0 80px", fontSize: "0.875rem" },
  { flex: "1 0 180px" },
  { flex: "2 2 60px" },
  { flex: "1 0 160px" },
  { flex: "0 0 auto" },
  { flex: "0 0 auto" },
];

function Name({ value }: CellProps<CronDatum, string>) {
  return (
    <div title={value} className="">
      {value}
    </div>
  );
}

function Schedule({
  value: { schedule },
}: CellProps<
  CronDatum,
  { schedule: CronSchedule; nextDate: Date | undefined }
>) {
  const literal = scheduleLiteral(schedule);

  let formattedSchedule = "";
  const wasmCron = useWasmCron();

  if (schedule.type === "interval") {
    const duration = formatDuration({ seconds: Number(schedule.seconds) });
    formattedSchedule = `Every ${duration}`;
  } else if (wasmCron) {
    const [cron, description] = wasmCron.parseAndDescribe(
      scheduleAsCron(schedule),
    );
    cron.free();
    formattedSchedule = prettierSaffron(description);
  }

  const tip = <pre className="text-left">{literal}</pre>;

  return (
    <div className="flex flex-col">
      <Tooltip tip={tip}>
        <div>{formattedSchedule}</div>
      </Tooltip>
    </div>
  );
}

function Function({ value }: CellProps<CronDatum, string>) {
  const url = useFunctionUrl(value);
  const name = displayName(value);
  return (
    <div className="truncate text-content-link hover:underline">
      <Link href={url} legacyBehavior>
        {name}
      </Link>
    </div>
  );
}

function PrevTs({
  date,
  run,
  isRunning,
}: {
  date?: Date;
  run?: CronJobLog;
  isRunning: boolean;
}) {
  if (!date || !run) return null;
  const message = `${
    isRunning
      ? "Started"
      : run.status.type === "success"
        ? "Success"
        : run.status.type === "err"
          ? "Failure"
          : "Run skipped"
  } `;
  return (
    <div className="flex flex-row truncate">
      <LiveTimestampDistance date={date} prefix={message} className="ml-1" />
    </div>
  );
}

function NextTs({ value }: { value: Date }) {
  return (
    <div className="flex flex-row truncate">
      <LiveTimestampDistance
        date={value}
        prefix={value < new Date() ? "Skipped run " : "Next run "}
        className="ml-1"
      />
    </div>
  );
}

function PrevNextTs({
  value,
}: CellProps<
  CronDatum,
  {
    nextDate: Date | undefined;
    prevDate: Date;
    prevRun: CronJobLog | undefined;
    nextRun: Doc<"_cron_next_run">;
  }
>) {
  const isRunning = value.nextRun.state.type === "inProgress";
  return (
    <div className="flex flex-col truncate">
      <PrevTs date={value.prevDate} isRunning={isRunning} run={value.prevRun} />
      {value.nextDate && <NextTs value={value.nextDate} />}
    </div>
  );
}

function More({ value }: CellProps<CronDatum, string>) {
  const router = useRouter();
  const handleClick = () => {
    router.query.id = value;
    void router.push({ query: router.query });
  };
  return (
    <Button
      onClick={handleClick}
      aria-label="show details"
      size="sm"
      variant="neutral"
      inline
      icon={<ChevronRightIcon aria-hidden />}
    />
  );
}

function Args({ value }: CellProps<CronDatum, JSONValue[]>) {
  const [showArgs, setShowArgs] = useState(false);

  if (value.length === 0) {
    return <div className="h-6 w-24" />;
  }

  return (
    <>
      <Button
        variant="neutral"
        inline
        size="sm"
        onClick={() => setShowArgs(true)}
        icon={<ExternalLinkIcon />}
      >
        Arguments
      </Button>
      {showArgs && (
        <DetailPanel
          onClose={() => setShowArgs(false)}
          header="Cron job arguments"
          content={
            <div className="h-full rounded-sm p-4">
              <ReadonlyCode
                path="scheduling"
                code={`${prettier(`
                [${value
                  .map((arg) => stringifyValue(jsonToConvex(arg)))
                  .join(",")}]`).slice(0, -1)}
                `}
              />
            </div>
          }
        />
      )}
    </>
  );
}

function cronDatum(cronJob: CronJobWithRuns) {
  const { name, cronSpec, lastRun, nextRun } = cronJob;
  const nextDate = new Date(Number(nextRun.nextTs / BigInt("1000000")));
  const prevDate = lastRun && new Date(Number(lastRun.ts / BigInt("1000000")));
  return {
    name,
    schedule: { schedule: cronSpec.cronSchedule, nextDate },
    prevNextTs: {
      prevDate,
      nextDate,
      prevRun: lastRun,
      nextRun,
    },
    udfPath: cronSpec.udfPath,
    udfArgs:
      cronSpec.udfArgs &&
      (JSON.parse(
        Buffer.from(cronSpec.udfArgs).toString("utf8"),
      ) as JSONValue[]),
  };
}
type CronDatum = ReturnType<typeof cronDatum>;

export function CronsTable({ cronJobs }: { cronJobs: CronJobWithRuns[] }) {
  const columns = useMemo(
    () =>
      [
        { Header: "Name", accessor: "name", Cell: Name },
        { Header: "Schedule", accessor: "schedule", Cell: Schedule },
        { Header: "Function", accessor: "udfPath", Cell: Function },
        { Header: "Next/Last Run", accessor: "prevNextTs", Cell: PrevNextTs },
        { Header: "Args", accessor: "udfArgs", Cell: Args },
        { Header: "More", accessor: "name", id: "more", Cell: More },
      ] as const,
    [],
  );

  const data = useMemo(() => cronJobs.map(cronDatum), [cronJobs]);

  const { getTableProps, getTableBodyProps, rows, prepareRow } = useTable({
    columns: columns as any, // TODO(react-18-upgrade)
    data,
  });

  return (
    <Sheet padding={false} className="scrollbar overflow-x-auto">
      <div {...getTableProps()} className="mx-4 block min-w-[42rem]">
        <div {...getTableBodyProps()} className="divide-y">
          {rows.map((row) => {
            prepareRow(row);
            return (
              <div
                {...row.getRowProps()}
                className="flex items-stretch justify-start gap-2 py-3 text-xs text-content-primary"
              >
                {row.cells.map((cell, i) => (
                  <div
                    {...cell.getCellProps()}
                    style={COLUMN_STYLES[i]}
                    className="flex items-center overflow-hidden"
                  >
                    {cell.render("Cell")}
                  </div>
                ))}
              </div>
            );
          })}
        </div>
      </div>
    </Sheet>
  );
}
