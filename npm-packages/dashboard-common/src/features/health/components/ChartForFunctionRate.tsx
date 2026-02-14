import classNames from "classnames";
import { format } from "date-fns";
import { useState } from "react";
import {
  ResponsiveContainer,
  LineChart,
  XAxis,
  YAxis,
  Legend,
  Tooltip,
  Line,
  ReferenceLine,
  CartesianGrid,
} from "recharts";
import { ChartTooltip } from "@common/elements/ChartTooltip";
import { useDeploymentAuditLogs } from "@common/lib/useDeploymentAuditLog";
import { timeLabelForMinute, formatNumberCompact } from "@common/lib/format";
import { ChartData } from "@common/lib/charts/types";
import { DeploymentTimes } from "@common/features/health/components/DeploymentTimes";
import { Button } from "@ui/Button";
import { FunctionNameOption } from "@common/elements/FunctionNameOption";
import { LoadingTransition } from "@ui/Loading";
import { Spinner } from "@ui/Spinner";

export function ChartForFunctionRate({
  chartData,
  kind,
}: {
  chartData: ChartData | undefined | null;
  kind:
    | "cacheHitRate"
    | "failureRate"
    | "schedulerStatus"
    | "functionConcurrency"
    | "functionCalls";
}) {
  const [shown, setShown] = useState<string | null>(null);
  const [startDate] = useState(new Date(Date.now() - 3600 * 1000));
  const auditLogs = useDeploymentAuditLogs(startDate.getTime(), {
    actions: ["push_config", "push_config_with_components"],
  });
  const deploysByMinute = auditLogs?.map(
    (log) => {
      const creationDate = new Date(log._creationTime);

      return {
        hour: format(creationDate, "h:mm a"),
        timestamp: log._creationTime,
      };
    },
    {} as Record<string, { hour: string; timestamp: number }[]>,
  );
  return (
    <div className="h-full min-h-52 w-full [&_*]:!outline-none">
      <LoadingTransition
        loadingProps={{
          fullHeight: false,
          className: "h-full w-full",
          shimmer: false,
        }}
        loadingState={
          <div className="flex h-full w-full items-center justify-center">
            <Spinner className="m-auto size-12" />
          </div>
        }
      >
        {chartData === null ? (
          <div className="flex h-[11.25rem] w-full items-center justify-center px-12 text-center text-sm text-content-secondary">
            {`Data will appear here as your ${kind === "cacheHitRate" ? "queries" : "functions"} are called.`}
          </div>
        ) : chartData === undefined ? null : (
          <ResponsiveContainer width="95%" height="95%">
            <LineChart
              data={chartData.data}
              style={{
                fontSize: 11,
                outline: "none",
              }}
            >
              {/* Show a reference line for each time bucket that had a deployment */}
              {deploysByMinute?.map(({ hour, timestamp }) => (
                <ReferenceLine
                  key={timestamp}
                  x={hour}
                  stroke="var(--brand-yellow)"
                  strokeDasharray="3 3"
                />
              ))}
              <XAxis
                axisLine={false}
                tickLine={false}
                dataKey="time"
                strokeWidth={1}
                domain={["auto", "auto"]}
                minTickGap={25}
                className="stroke-content-secondary"
                tick={{ fontSize: 11, fill: "currentColor" }}
              />
              <YAxis
                axisLine={false}
                tickLine={false}
                width={40}
                tickFormatter={(value) =>
                  kind === "functionCalls"
                    ? formatNumberCompact(value as number)
                    : kind === "schedulerStatus" ||
                        kind === "functionConcurrency"
                      ? value
                      : `${value.toFixed((value as number) % 1 === 0 ? 0 : 2)}%`
                }
                domain={
                  kind !== "schedulerStatus" &&
                  kind !== "functionConcurrency" &&
                  kind !== "functionCalls"
                    ? [0, 100]
                    : undefined
                }
                ticks={
                  kind !== "schedulerStatus" &&
                  kind !== "functionConcurrency" &&
                  kind !== "functionCalls"
                    ? [0, 25, 50, 75, 100]
                    : undefined
                }
                interval={
                  kind === "schedulerStatus" ||
                  kind === "functionConcurrency" ||
                  kind === "functionCalls"
                    ? 0
                    : undefined
                }
                allowDecimals={kind !== "functionCalls"}
                className="stroke-content-secondary"
                tick={{ fontSize: 11, fill: "currentColor" }}
              />
              <Legend
                align="left"
                verticalAlign="bottom"
                iconType="plainline"
                iconSize={12}
                layout="horizontal"
                formatter={(_value, entry, idx) => {
                  const { dataKey } = entry;
                  return (
                    <Button
                      variant="unstyled"
                      key={idx}
                      className={classNames(
                        "span items-center gap-1 transition-opacity text-content-primary",
                        shown === dataKey || shown === null
                          ? "opacity-100"
                          : "opacity-50",
                      )}
                      onClick={() =>
                        shown === dataKey
                          ? setShown(null)
                          : setShown(dataKey as string)
                      }
                    >
                      {dataKey === "_rest" ? (
                        `All${[].length > 1 ? " other" : ""} ${kind === "cacheHitRate" ? "queries" : "functions"}`
                      ) : kind === "schedulerStatus" ? (
                        "Lag Time (minutes)"
                      ) : kind === "functionConcurrency" ? (
                        (dataKey as string)
                      ) : kind === "functionCalls" ? (
                        <FunctionNameOption
                          maxChars={24}
                          label={dataKey as string}
                        />
                      ) : (
                        <FunctionNameOption
                          maxChars={24}
                          label={dataKey as string}
                        />
                      )}
                    </Button>
                  );
                }}
              />

              <Tooltip
                animationDuration={100}
                content={({ active, payload, label }) => {
                  const deploymentTimes = deploysByMinute
                    ?.filter((deploy) => deploy.hour === label)
                    .map((deploy) => format(new Date(deploy.timestamp), "pp"));
                  return (
                    <ChartTooltip
                      active={active}
                      payload={payload
                        ?.filter(
                          ({ dataKey }) => shown === dataKey || shown === null,
                        )
                        .map((dataPoint) => ({
                          ...dataPoint,
                          formattedValue: (
                            <span className="flex min-w-48 items-center justify-between">
                              <div>
                                {dataPoint.dataKey === "_rest" ? (
                                  `All${payload.length > 1 ? " other" : ""} ${kind === "cacheHitRate" ? "queries" : "functions"}`
                                ) : kind === "schedulerStatus" ? (
                                  "Lag Time"
                                ) : kind === "functionConcurrency" ? (
                                  (dataPoint.dataKey as string)
                                ) : kind === "functionCalls" ? (
                                  <FunctionNameOption
                                    maxChars={24}
                                    label={dataPoint.dataKey as string}
                                  />
                                ) : (
                                  <FunctionNameOption
                                    maxChars={24}
                                    label={dataPoint.dataKey as string}
                                  />
                                )}
                              </div>
                              <div>
                                {kind === "schedulerStatus"
                                  ? `${(dataPoint.value as number).toLocaleString()} minutes`
                                  : kind === "functionConcurrency" ||
                                      kind === "functionCalls"
                                    ? `${formatNumberCompact(dataPoint.value as number)} ${(dataPoint.value as number) === 1 ? "call" : "calls"}`
                                    : `${(dataPoint.value as number).toFixed(
                                        (dataPoint.value as number) % 1 === 0
                                          ? 0
                                          : 2,
                                      )}%`}
                              </div>
                            </span>
                          ),
                        }))}
                      extraContent={
                        <DeploymentTimes deploymentTimes={deploymentTimes} />
                      }
                      label={timeLabelForMinute(label)}
                      showLegend
                    />
                  );
                }}
              />
              <CartesianGrid
                className="stroke-content-tertiary/40"
                horizontal
                strokeWidth={1}
                vertical={false}
                verticalFill={[]}
                syncWithTicks
                horizontalValues={
                  kind !== "schedulerStatus" &&
                  kind !== "functionConcurrency" &&
                  kind !== "functionCalls"
                    ? [0, 25, 50, 75, 100]
                    : undefined
                }
              />

              {chartData.lineKeys.map((line) => {
                const dataKey = line.key;
                const { name } = line;
                const { color } = line;
                return (
                  <Line
                    isAnimationActive={false}
                    strokeWidth={1.5}
                    activeDot={{
                      r: 4,
                      className: "stroke-none",
                      display:
                        shown === dataKey || shown === null ? "block" : "none",
                    }}
                    key={dataKey}
                    dataKey={dataKey}
                    name={name}
                    min={0}
                    display={
                      shown === dataKey || shown === null ? "block" : "none"
                    }
                    className="transition-opacity"
                    stroke={color}
                    fillOpacity={1}
                    fill={`url(#${dataKey})`}
                    dot={false}
                  />
                );
              })}
            </LineChart>
          </ResponsiveContainer>
        )}
      </LoadingTransition>
    </div>
  );
}
