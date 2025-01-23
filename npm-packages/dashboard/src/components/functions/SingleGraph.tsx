import React, { useEffect, useState } from "react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from "recharts";

import {
  ChartData,
  ChartDataSource,
  LoadingTransition,
  timeLabel,
  ChartTooltip,
  HealthCard,
} from "dashboard-common";

export function SingleGraph({
  title,
  dataSource,
  syncId,
}: {
  title: "Cache Hit Rate" | "Function Calls" | "Errors" | "Execution Time";
  dataSource: ChartDataSource;
  syncId?: string;
}) {
  const [chartData, setChartData] = useState<ChartData>();

  useEffect(() => {
    async function getChartData() {
      const initEndDate = new Date();
      const initStartDate = new Date(initEndDate);
      initStartDate.setHours(initStartDate.getHours() - 1);
      setChartData(await dataSource(initStartDate, initEndDate));
    }
    void getChartData();
  }, [dataSource]);

  const force = Math.random();

  return (
    <HealthCard title={title}>
      <div className="min-h-52 w-full">
        <LoadingTransition
          loadingProps={{
            fullHeight: false,
            className: "h-[13rem]",
          }}
        >
          {chartData && (
            <ResponsiveContainer width="95%" height="100%">
              <LineChart
                syncId={syncId}
                key={force}
                data={chartData.data}
                style={{
                  fontSize: 11,
                  left: -24,
                }}
              >
                <XAxis
                  dataKey="time"
                  tickLine={false}
                  axisLine={false}
                  strokeWidth={1}
                  domain={["auto", "auto"]}
                  minTickGap={25}
                  className="stroke-content-secondary"
                  tick={{ fontSize: 11, fill: "currentColor" }}
                />
                <YAxis
                  axisLine={false}
                  tickLine={false}
                  ticks={
                    title === "Cache Hit Rate"
                      ? [0, 25, 50, 75, 100]
                      : undefined
                  }
                  tickFormatter={(value) =>
                    new Intl.NumberFormat("en-US", {
                      notation: "compact",
                      compactDisplay: "short",
                    }).format(value) + (title === "Cache Hit Rate" ? "%" : "")
                  }
                  className="stroke-content-secondary"
                  tick={{ fontSize: 11, fill: "currentColor" }}
                  width={60}
                />
                <Tooltip
                  content={({ active, payload, label }) => (
                    <ChartTooltip
                      active={active}
                      payload={payload}
                      label={timeLabel(label)}
                      showLegend
                    />
                  )}
                  animationDuration={100}
                />
                <CartesianGrid
                  className="stroke-content-tertiary/40"
                  horizontal
                  strokeWidth={1}
                  vertical={false}
                  verticalFill={[]}
                  syncWithTicks
                />
                {chartData.lineKeys.map((line) => {
                  const dataKey = line.key;
                  const { name } = line;
                  const { color } = line;
                  return (
                    <Line
                      isAnimationActive={false}
                      strokeWidth={1.5}
                      activeDot={{ r: 4, className: "stroke-none" }}
                      key={dataKey}
                      dataKey={dataKey}
                      name={name}
                      min={0}
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
    </HealthCard>
  );
}
