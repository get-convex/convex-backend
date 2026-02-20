import { useMemo, useRef, useState, useEffect } from "react";
import { createPortal } from "react-dom";
import { BarChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import type { TooltipContentProps } from "recharts";
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

function PortalTooltip({
  active,
  payload,
  label,
  coordinate,
  content,
  chartRef,
  hideTooltip,
}: any) {
  const tooltipRef = useRef<HTMLDivElement>(null);
  const [tooltipSize, setTooltipSize] = useState<{
    width: number;
    height: number;
  } | null>(null);

  // Measure tooltip size after render
  useEffect(() => {
    if (tooltipRef.current) {
      const { width, height } = tooltipRef.current.getBoundingClientRect();
      setTooltipSize({ width, height });
    }
  }, [active, payload, label]);

  if (!active || !coordinate || hideTooltip) return null;

  // The coordinate is relative to the chart SVG, so we need to convert it to viewport coordinates
  // by adding the chart's position on the page
  const rect = chartRef?.current?.getBoundingClientRect();

  const offset = 10;
  let left = (rect?.left || 0) + coordinate.x + offset;
  let top = (rect?.top || 0) + coordinate.y + offset;

  // Use actual tooltip dimensions if available, otherwise use estimates
  const tooltipWidth = tooltipSize?.width || 300;
  const tooltipHeight = tooltipSize?.height || 150;
  const viewportPadding = 20;

  // Check if tooltip would overflow right edge (with some padding)
  if (left + tooltipWidth > window.innerWidth - viewportPadding) {
    // Position to the left of cursor
    left = (rect?.left || 0) + coordinate.x - offset - tooltipWidth;
    // Ensure it doesn't go off the left edge
    left = Math.max(offset, left);
  }

  // Check if tooltip would overflow bottom edge (with some padding)
  if (top + tooltipHeight > window.innerHeight - viewportPadding) {
    top = (rect?.top || 0) + coordinate.y - offset - tooltipHeight;
    // Ensure it doesn't go off the top edge
    top = Math.max(offset, top);
  }

  return createPortal(
    <div
      ref={tooltipRef}
      style={{
        position: "fixed",
        left,
        top,
        pointerEvents: "none",
        zIndex: 9999,
        fontSize: "12px",
      }}
    >
      {content({ active, payload, label })}
    </div>,
    document.body,
  );
}

export function DailyChart({
  data,
  showCategoryInTooltip = false,
  children,
  quantityType,
  colorMap,
  yAxisWidth = 60,
  customTooltip,
  hideTooltip = false,
}: React.PropsWithChildren<{
  data: { dateNumeric: number }[];
  categoryInTooltip?: boolean;
  showCategoryInTooltip?: boolean;
  quantityType: QuantityType;
  colorMap?: Map<string, string>;
  yAxisWidth?: number;
  customTooltip?: (
    props: TooltipContentProps<any, any>,
  ) => React.ReactElement | null;
  hideTooltip?: boolean;
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
  const chartRef = useRef<HTMLDivElement>(null);
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
      <div ref={chartRef} style={{ width: "100%", height: "100%" }}>
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
              content={(props) => (
                <PortalTooltip
                  {...props}
                  chartRef={chartRef}
                  hideTooltip={hideTooltip}
                  content={
                    customTooltip ||
                    (({ active, payload, label }: any) => (
                      <ChartTooltip
                        active={active}
                        payload={payload
                          ?.filter((dataPoint: any) => {
                            const value = dataPoint.value as number;
                            return value > 0;
                          })
                          .map((dataPoint: any) => {
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
                                prefix +
                                formatQuantity(value, quantityType) +
                                suffix,
                            };
                          })
                          .reverse()}
                        label={
                          label !== null && label !== undefined
                            ? new Date(label).toLocaleDateString("en-us", {
                                year: "numeric",
                                month: "long",
                                day: "numeric",
                                timeZone: "UTC",
                              })
                            : ""
                        }
                        showLegend={showCategoryInTooltip}
                      />
                    ))
                  }
                />
              )}
              labelClassName="font-semibold"
            />
            {children}
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
