import {
  InsightsSummaryData,
  useOCCByHour,
  useBytesReadAverageByHour,
  useBytesReadCountByHour,
  useInsightsPeriod,
  useDocumentsReadAverageByHour,
  useDocumentsReadCountByHour,
} from "api/insights";
import {
  formatBytes,
  formatNumberCompact,
  toNumericUTC,
} from "dashboard-common/lib/format";
import { ChartTooltip } from "dashboard-common/elements/ChartTooltip";
import { format } from "date-fns";
import {
  ResponsiveContainer,
  XAxis,
  YAxis,
  Tooltip,
  Line,
  LineChart,
  ReferenceLine,
  CartesianGrid,
} from "recharts";
import { LoadingTransition } from "dashboard-common/elements/Loading";
import { DeploymentTimes } from "dashboard-common/features/health/components/DeploymentTimes";
import { useDeploymentAuditLogs } from "dashboard-common/lib/useDeploymentAuditLog";
import { documentsReadLimit, megabytesReadLimit } from "./ProblemForInsight";

export function ChartForInsight({ insight }: { insight: InsightsSummaryData }) {
  switch (insight.kind) {
    case "occFailedPermanently":
    case "occRetried":
      return <ChartOCC insight={insight} />;
    case "bytesReadAverageThreshold":
      return <ChartAverageBytesRead insight={insight} />;
    case "bytesReadCountThreshold":
      return <ChartCountBytesRead insight={insight} />;
    case "docsReadAverageThreshold":
      return <ChartAverageDocumentsRead insight={insight} />;
    case "docsReadCountThreshold":
      return <ChartCountDocumentsRead insight={insight} />;
    default: {
      const _exhaustiveCheck: never = insight;
      return null;
    }
  }
}

function InsightsLineChart<T extends Record<string, any>>({
  data,
  name,
  dataKey,
  max,
  formatY = formatNumberCompact,
}: {
  data?: T[];
  name: string;
  dataKey: keyof T extends string | number ? keyof T : never;
  max?: number;
  formatY?: (value: number) => string;
}) {
  const { from } = useInsightsPeriod();
  const auditLogs = useDeploymentAuditLogs(toNumericUTC(from), {
    actions: ["push_config", "push_config_with_components"],
  });
  const deploysByHour = auditLogs?.map(
    (log) => {
      const creationDate = new Date(log._creationTime);
      const hour = `${creationDate.getUTCFullYear()}-${String(
        creationDate.getUTCMonth() + 1,
      ).padStart(2, "0")}-${String(creationDate.getUTCDate()).padStart(
        2,
        "0",
      )} ${String(creationDate.getUTCHours()).padStart(2, "0")}:00:00`;
      return { hour, timestamp: log._creationTime };
    },

    {} as Record<string, { hour: string; timestamp: number }[]>,
  );

  return (
    <div>
      <LoadingTransition
        loadingProps={{ fullHeight: false, className: "h-[200px] w-full" }}
      >
        {data && (
          <ResponsiveContainer height={200} width="100%">
            <LineChart
              data={data}
              style={{
                fontSize: 12,
              }}
            >
              {/* Show a reference line for each time bucket that had a deployment */}
              {deploysByHour?.map(({ hour, timestamp }) => (
                <ReferenceLine
                  key={timestamp}
                  x={hour}
                  stroke="rgb(var(--brand-yellow))"
                  strokeDasharray="3 3"
                />
              ))}

              <CartesianGrid
                className="stroke-content-tertiary/40"
                horizontal
                strokeWidth={1}
                vertical={false}
                verticalFill={[]}
                horizontalFill={["rgba(var(--background-tertiary), 0.33)"]}
                syncWithTicks
              />

              <XAxis
                dataKey="dateHour"
                domain={["auto", "auto"]}
                tickFormatter={dateLabel}
                strokeWidth={1}
                className="text-content-secondary"
                axisLine={{
                  stroke: "currentColor",
                }}
                tickLine={{
                  stroke: "currentColor",
                }}
                tick={{
                  fontSize: 12,
                  fill: "currentColor",
                }}
                max={max}
                ticks={data
                  .filter(
                    (d) =>
                      new Date(toNumericUTCWithHour(d.dateHour)).getHours() ===
                      0,
                  )
                  .map((d) => d.dateHour)}
              />
              <YAxis
                tick={{
                  fontSize: 12,
                  className: "",
                  fill: "currentColor",
                }}
                className="text-content-secondary"
                axisLine={{
                  stroke: "currentColor",
                }}
                tickLine={false}
                tickFormatter={formatY}
                width={48}
              />
              <Tooltip
                content={({ active, payload, label }) => {
                  const deploymentTimes = deploysByHour
                    ?.filter((deploy) => deploy.hour === label)
                    .map((deploy) => format(new Date(deploy.timestamp), "Pp"));
                  return (
                    <ChartTooltip
                      active={active}
                      payload={payload?.map((p) => ({
                        formattedValue: (
                          <div className="flex flex-col items-start">
                            <div>
                              {formatY(p.payload[dataKey])} {name}
                            </div>
                          </div>
                        ),
                        ...p,
                      }))}
                      extraContent={
                        <DeploymentTimes deploymentTimes={deploymentTimes} />
                      }
                      label={timeLabel(label)}
                    />
                  );
                }}
                animationDuration={100}
              />
              <Line
                isAnimationActive={false}
                className="stroke-chart-line-1"
                activeDot={{ r: 4, className: "stroke-none" }}
                name={name}
                dataKey={dataKey}
                dot={false}
              />
            </LineChart>
          </ResponsiveContainer>
        )}
      </LoadingTransition>
    </div>
  );
}

function ChartOCC({
  insight,
}: {
  insight: InsightsSummaryData & {
    kind: "occFailedPermanently" | "occRetried";
  };
}) {
  const data = useOCCByHour({
    functionId: insight.functionId,
    componentPath: insight.componentPath,
    tableName: insight.occTableName,
    permanentFailure: insight.kind === "occFailedPermanently",
  });

  return (
    <InsightsLineChart
      data={data}
      dataKey="occCalls"
      name={`write conflict ${insight.kind === "occFailedPermanently" ? "failures" : "retries"}`}
    />
  );
}

function ChartAverageBytesRead({
  insight,
}: {
  insight: InsightsSummaryData & { kind: "bytesReadAverageThreshold" };
}) {
  const data = useBytesReadAverageByHour({
    functionId: insight.functionId,
    componentPath: insight.componentPath,
  });

  return (
    <InsightsLineChart
      data={data}
      dataKey="avg"
      name="read on average"
      max={megabytesReadLimit * 1024 * 1024}
      formatY={formatBytes}
    />
  );
}

function ChartCountBytesRead({
  insight,
}: {
  insight: InsightsSummaryData & { kind: "bytesReadCountThreshold" };
}) {
  const data = useBytesReadCountByHour({
    functionId: insight.functionId,
    componentPath: insight.componentPath,
  });

  return (
    <InsightsLineChart
      data={data}
      dataKey="count"
      name={`function calls reading more than ${formatBytes(8 * 1024 * 1024 * 0.8)}`}
    />
  );
}

function ChartAverageDocumentsRead({
  insight,
}: {
  insight: InsightsSummaryData & { kind: "docsReadAverageThreshold" };
}) {
  const data = useDocumentsReadAverageByHour({
    functionId: insight.functionId,
    componentPath: insight.componentPath,
  });

  return (
    <InsightsLineChart
      data={data}
      dataKey="avg"
      name="documents read on average"
      max={documentsReadLimit * 0.8}
      formatY={formatNumberCompact}
    />
  );
}

function ChartCountDocumentsRead({
  insight,
}: {
  insight: InsightsSummaryData & { kind: "docsReadCountThreshold" };
}) {
  const data = useDocumentsReadCountByHour({
    functionId: insight.functionId,
    componentPath: insight.componentPath,
  });

  return (
    <InsightsLineChart
      data={data}
      dataKey="count"
      name={`function calls reading more than ${formatNumberCompact(16384 * 0.8)} documents`}
    />
  );
}

const dateLabel = (value: string) => {
  if (!value) {
    return "";
  }
  const date = new Date(toNumericUTCWithHour(value));
  return date.toLocaleDateString();
};

const timeLabel = (value: string) => {
  if (!value) {
    return "";
  }
  const date = new Date(toNumericUTCWithHour(value));
  const oneHourLater = new Date(date.getTime() + 60 * 60 * 1000);

  return `${format(date, "P")} ${format(date, "h a")} – ${format(oneHourLater, "h a")}`;
};

function toNumericUTCWithHour(dateString: string) {
  // Parsing manually the date to use UTC.
  const [datePart, timePart] = dateString.split(" ");
  const [year, month, day] = datePart.split("-");
  const [hour = "0", minute = "0", second = "0"] = timePart
    ? timePart.split(":")
    : [];
  return Date.UTC(
    Number(year),
    Number(month) - 1,
    Number(day),
    Number(hour),
    Number(minute),
    Number(second),
  );
}
