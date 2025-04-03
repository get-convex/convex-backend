import { useContext, useEffect, useState } from "react";
import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { LoadingTransition } from "@common/elements/Loading";
import { ChartData, ChartDataSource } from "@common/lib/charts/types";
import { Callout } from "@common/elements/Callout";
import { ChartTooltip } from "@common/elements/ChartTooltip";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

export function BigChart({
  dataSources,
  labels,
  syncId,
}: {
  dataSources: ChartDataSource[];
  labels: string[];
  syncId: string;
}) {
  const [chartData, setChartData] = useState<(ChartData | undefined)[]>();

  const initEndDate = new Date();
  const initStartDate = new Date(initEndDate);
  initStartDate.setHours(initStartDate.getHours() - 1);

  const [startDate] = useState(initStartDate);
  const [endDate] = useState(initEndDate);

  const { captureException } = useContext(DeploymentInfoContext);

  useEffect(() => {
    async function getChartData() {
      if (startDate < endDate) {
        const data = await Promise.allSettled(
          dataSources.map(async (dataSource) => dataSource(startDate, endDate)),
        );

        setChartData(
          data.map((d) => (d.status === "fulfilled" ? d.value : undefined)),
        );

        // Make sure we log any errors fetching data.
        data
          .filter<PromiseRejectedResult>(
            (d): d is PromiseRejectedResult => d.status === "rejected",
          )
          .forEach((d) => {
            captureException(d.reason);
          });
      }
    }
    void getChartData();
  }, [dataSources, startDate, endDate, captureException]);

  return (
    <div className="flex flex-col gap-6 pt-4">
      <LoadingTransition
        loadingProps={{
          className: dataSources.length > 1 ? "h-[31rem]" : "h-[14.75rem]",
          fullHeight: false,
        }}
      >
        {chartData && (
          <div className="flex w-full flex-col gap-2">
            {chartData.map((chart, i) =>
              chart === undefined ? (
                <Callout key={i} variant="error">
                  Failed to load {labels[i]} data
                </Callout>
              ) : (
                <div key={i} className="flex w-full flex-col gap-4">
                  <h5>{labels[i]}</h5>
                  <ResponsiveContainer height={200} width="100%">
                    <LineChart
                      key={i}
                      data={chart.data}
                      syncId={syncId}
                      style={{
                        fontSize: 12,
                      }}
                    >
                      <XAxis
                        dataKey={chart.xAxisKey}
                        axisLine={{
                          stroke: "currentColor",
                        }}
                        tickLine={{
                          stroke: "currentColor",
                        }}
                        domain={["auto", "auto"]}
                        className="text-content-secondary"
                        strokeWidth={1}
                        minTickGap={100}
                        tick={{ fontSize: 12, fill: "currentColor" }}
                      />
                      <YAxis
                        axisLine={{
                          stroke: "currentColor",
                        }}
                        tickLine={false}
                        className="text-content-secondary"
                        tick={{ fontSize: 12, fill: "currentColor" }}
                        width={60}
                      />
                      <CartesianGrid
                        className="stroke-content-tertiary/40"
                        horizontal
                        strokeWidth={1}
                        vertical={false}
                        verticalFill={[]}
                        horizontalFill={[
                          "rgba(var(--background-tertiary), 0.33)",
                        ]}
                        syncWithTicks
                      />
                      <Tooltip
                        content={({ active, payload, label }) => (
                          <ChartTooltip
                            active={active}
                            payload={payload}
                            label={timeLabel(label)}
                          />
                        )}
                        animationDuration={100}
                      />
                      {chart.lineKeys.length! > 1 ? (
                        <Legend
                          iconType="circle"
                          iconSize={10}
                          content={(props) => {
                            // eslint-disable-next-line react/prop-types
                            const { payload } = props;
                            if (!payload) {
                              return null;
                            }

                            return (
                              <ul className="flex w-full justify-center gap-2">
                                {/* eslint-disable-next-line react/prop-types */}
                                {payload.map((entry, index) => (
                                  <li
                                    key={`item-${index}`}
                                    className="flex items-center gap-1 text-content-primary"
                                  >
                                    <span
                                      style={{ backgroundColor: entry.color }}
                                      className="h-2 w-2 rounded-full"
                                    />
                                    {entry.value}
                                  </li>
                                ))}
                              </ul>
                            );
                          }}
                        />
                      ) : (
                        <div />
                      )}
                      {chart.lineKeys.map((line) => {
                        const dataKey = line.key;
                        const { name } = line;
                        const { color } = line;
                        return (
                          <Line
                            isAnimationActive={false}
                            strokeWidth={1}
                            activeDot={{ r: 4, className: "stroke-none" }}
                            key={dataKey}
                            dataKey={dataKey}
                            name={name}
                            stroke={color}
                            fillOpacity={1}
                            min={0}
                            fill={`url(#${dataKey})`}
                            dot={false}
                          />
                        );
                      })}
                    </LineChart>
                  </ResponsiveContainer>
                </div>
              ),
            )}
          </div>
        )}
      </LoadingTransition>
    </div>
  );
}

export const timeLabel = (value: string) => {
  if (!value) {
    return "";
  }
  // TODO(ari): Consolidate all the time rendering logic - this is a hack
  // for now
  if (value.includes("-") || !value.includes(":")) {
    return value;
  }
  const [time, modifier] = value.split(" ");
  const [hours, minutes] = time.split(":");
  const date = new Date();
  date.setHours(parseInt(hours) + (modifier === "PM" ? 12 : 0));
  date.setMinutes(parseInt(minutes));
  const oneMinuteLater = new Date(date);
  oneMinuteLater.setMinutes(date.getMinutes() + 1);

  return `${formatTime(date)} – ${formatTime(oneMinuteLater)}`;
};

const formatTime = (date: Date) => {
  let hours = date.getHours();
  const minutes = date.getMinutes();
  const ampm = hours >= 12 ? "PM" : "AM";
  hours %= 12;
  hours = hours || 12; // the hour '0' should be '12'
  const strMinutes = minutes < 10 ? `0${minutes}` : minutes;
  return `${hours}:${strMinutes} ${ampm}`;
};
