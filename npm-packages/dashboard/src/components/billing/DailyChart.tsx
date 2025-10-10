import { useMemo } from "react";
import { BarChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { useMeasure } from "react-use";
import { ChartTooltip } from "@common/elements/ChartTooltip";
import {
  QuantityType,
  formatQuantity,
  formatQuantityCompact,
} from "./lib/formatQuantity";

// To avoid having a bar displayed too wide, we set a minimum amount of days for the chart's x-axis span.
const MIN_DAY_SPAN = 6;

const MS_IN_DAY = 24 * 60 * 60 * 1000;

export function DailyChart({
  data,
  showCategoryInTooltip = false,
  children,
  quantityType,
  colorMap,
  yAxisWidth = 60,
}: React.PropsWithChildren<{
  data: { dateNumeric: number }[];
  categoryInTooltip?: boolean;
  showCategoryInTooltip?: boolean;
  quantityType: QuantityType;
  colorMap?: Map<string, string>;
  yAxisWidth?: number;
}>) {
  const { daysWithValues, minDate, daysCount } = useMemo(() => {
    const values = new Set(data.map(({ dateNumeric }) => dateNumeric));
    const min = Math.min(...values);
    const max = Math.max(...values);
    return {
      daysWithValues: values,
      minDate: min,
      daysCount: Math.max(MIN_DAY_SPAN, (max - min) / MS_IN_DAY) + 1,
    };
  }, [data]);

  const [containerRef, { width: containerWidth }] =
    useMeasure<HTMLDivElement>();
  const ticks = useMemo(() => {
    if (containerWidth === 0) {
      return [];
    }

    const graphMargin = 90;
    const minBarWidth = 50;

    const barsWidth = containerWidth - graphMargin;
    const dayWidth = barsWidth / daysCount;
    const daysByTick = Math.ceil(minBarWidth / dayWidth);
    const ticksCount = Math.ceil(daysCount / daysByTick);

    return [...Array(ticksCount).keys()]
      .map((i) => minDate + i * daysByTick * MS_IN_DAY)
      .filter((day) => daysWithValues.has(day));
  }, [containerWidth, daysCount, minDate, daysWithValues]);

  return (
    <div ref={containerRef} className="h-full animate-fadeInFromLoading">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={data} className="text-xs text-content-primary">
          <XAxis
            scale="time"
            type="number"
            domain={[
              minDate - MS_IN_DAY / 2,
              minDate + (daysCount - 1) * MS_IN_DAY + MS_IN_DAY / 2,
            ]}
            axisLine={false}
            tickSize={0}
            tick={{
              fill: "currentColor",
            }}
            ticks={ticks}
            dataKey="dateNumeric"
            padding={{ left: 12 }}
            tickFormatter={(dateNumeric) =>
              new Date(dateNumeric).toLocaleDateString("en-us", {
                month: "short",
                day: "numeric",
                timeZone: "UTC",
              })
            }
          />
          <YAxis
            axisLine={false}
            tickSize={0}
            tickFormatter={(value) =>
              formatQuantityCompact(value, quantityType)
            }
            padding={{ top: 8, bottom: 8 }}
            tick={{
              fill: "currentColor",
            }}
            style={{
              fontVariantNumeric: "tabular-nums",
            }}
            width={yAxisWidth}
          />
          <Tooltip
            isAnimationActive={false}
            cursor={{
              fill: undefined, // Set in globals.css
            }}
            content={({ active, payload, label }) => (
              <ChartTooltip
                active={active}
                payload={payload
                  ?.filter((dataPoint) => {
                    const value = dataPoint.value as number;
                    return value > 0;
                  })
                  .map((dataPoint) => {
                    const prefix = showCategoryInTooltip
                      ? `${dataPoint.name}: `
                      : "";
                    const value = dataPoint.value as number;
                    const suffix =
                      !showCategoryInTooltip && quantityType === "unit"
                        ? ` ${dataPoint.name}`
                        : "";
                    const className = colorMap?.get(
                      dataPoint.dataKey as string,
                    );
                    return {
                      ...dataPoint,
                      ...(className && { className }),
                      formattedValue:
                        prefix + formatQuantity(value, quantityType) + suffix,
                    };
                  })
                  .reverse()}
                label={new Date(label).toLocaleDateString("en-us", {
                  year: "numeric",
                  month: "long",
                  day: "numeric",
                  timeZone: "UTC",
                })}
                showLegend={showCategoryInTooltip}
              />
            )}
            labelClassName="font-semibold"
          />
          {children}
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}
