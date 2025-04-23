import { useInsightsPeriod, Insight } from "api/insights";
import {
  formatBytes,
  formatNumberCompact,
  toNumericUTC,
} from "@common/lib/format";
import { ChartTooltip } from "@common/elements/ChartTooltip";
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
import { LoadingTransition } from "@ui/Loading";
import { DeploymentTimes } from "@common/features/health/components/DeploymentTimes";
import { useDeploymentAuditLogs } from "@common/lib/useDeploymentAuditLog";
import { documentsReadLimit, megabytesReadLimit } from "./ProblemForInsight";

export function ChartForInsight({ insight }: { insight: Insight }) {
  switch (insight.kind) {
    case "occFailedPermanently":
    case "occRetried":
      return <ChartOCC insight={insight} />;
    case "bytesReadLimit":
    case "bytesReadThreshold": {
      const bytesReadInsight = insight as Insight & {
        kind: "bytesReadLimit" | "bytesReadThreshold";
      };
      return <ChartCountBytesRead insight={bytesReadInsight} />;
    }
    case "documentsReadLimit":
    case "documentsReadThreshold": {
      const docsReadInsight = insight as Insight & {
        kind: "documentsReadLimit" | "documentsReadThreshold";
      };
      return <ChartCountDocumentsRead insight={docsReadInsight} />;
    }
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
                dataKey="hour"
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
                      new Date(toNumericUTCWithHour(d.hour)).getHours() === 0,
                  )
                  .map((d) => d.hour)}
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
  insight: Insight & { kind: "occFailedPermanently" | "occRetried" };
}) {
  return (
    <InsightsLineChart
      data={insight.details.hourlyCounts}
      dataKey="count"
      name={`write conflict ${insight.kind === "occFailedPermanently" ? "failures" : "retries"}`}
    />
  );
}

function ChartCountBytesRead({
  insight,
}: {
  insight: Insight & { kind: "bytesReadLimit" | "bytesReadThreshold" };
}) {
  return (
    <InsightsLineChart
      data={insight.details.hourlyCounts}
      dataKey="count"
      name={`function calls reading more than ${formatBytes(megabytesReadLimit * 1024 * 1024 * 0.8)}`}
    />
  );
}

function ChartCountDocumentsRead({
  insight,
}: {
  insight: Insight & {
    kind: "documentsReadLimit" | "documentsReadThreshold";
  };
}) {
  return (
    <InsightsLineChart
      data={insight.details.hourlyCounts}
      dataKey="count"
      name={`function calls reading more than ${formatNumberCompact(documentsReadLimit * 0.8)} documents`}
    />
  );
}

function toNumericUTCWithHour(dateString: string) {
  try {
    // Handle ISO-8601 format (with T separator)
    if (dateString.includes("T")) {
      const [datePart, timePart] = dateString.split("T");
      const [year, month, day] = datePart.split("-");
      const hour = timePart;

      return Date.UTC(
        Number(year),
        Number(month) - 1,
        Number(day),
        Number(hour),
        0,
        0,
      );
    }

    // Regular format with space separator
    const [datePart, timePart] = dateString.split(" ");
    if (!datePart) return NaN; // Invalid date

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
  } catch (error) {
    console.error("Error parsing date:", error, dateString);
    return NaN; // Return NaN for invalid dates
  }
}

const dateLabel = (value: string) => {
  if (!value) {
    return "";
  }

  try {
    const timestamp = toNumericUTCWithHour(value);
    if (Number.isNaN(timestamp)) {
      console.warn("Invalid date for dateLabel:", value);
      return "Invalid date";
    }

    const date = new Date(timestamp);
    return date.toLocaleDateString();
  } catch (error) {
    console.error("Error in dateLabel:", error, value);
    return "Invalid date";
  }
};

const timeLabel = (value: string) => {
  if (!value) {
    return "";
  }

  try {
    const timestamp = toNumericUTCWithHour(value);
    if (Number.isNaN(timestamp)) {
      console.warn("Invalid date for timeLabel:", value);
      return "Invalid date";
    }

    const date = new Date(timestamp);
    const oneHourLater = new Date(date.getTime() + 60 * 60 * 1000);

    return `${format(date, "P")} ${format(date, "h a")} – ${format(oneHourLater, "h a")}`;
  } catch (error) {
    console.error("Error formatting date:", error, value);
    return "Invalid date format";
  }
};
