import * as Sentry from "@sentry/nextjs";
import { DailyMetric, DailyPerTagMetrics } from "hooks/usageMetrics";
import {
  Bar,
  BarChart,
  Legend,
  Rectangle,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { useMemo } from "react";
import { ChartTooltip, toNumericUTC } from "dashboard-common";
import groupBy from "lodash/groupBy";
import sumBy from "lodash/sumBy";
import {
  QuantityType,
  formatQuantity,
  formatQuantityCompact,
} from "data/Charts/formatQuantity";
import { useMeasure } from "react-use";
import { UsageNoDataError } from "./TeamUsageError";

// To avoid having a bar displayed too wide, we set a minimum amount of days for the chart’s x-axis span.
const MIN_DAY_SPAN = 6;

// When there is only a data point, we have to set the bar width manually to make it appear (https://github.com/recharts/recharts/issues/3640).
// This value has been measured manually on a desktop screen size, but it should also look good in other contexts where there is only one bar.
const SINGLE_BAR_WIDTH = 91;

const MS_IN_DAY = 24 * 60 * 60 * 1000;

export function UsageStackedBarChart({
  rows,
  entity,
  categories,
  categoryRenames = {},
  quantityType = "unit",
  showCategoryTotals = true,
}: {
  rows: DailyPerTagMetrics[];
  entity: string;
  categories: {
    [tag: string]: {
      name: string;
      color: string;
    };
  };
  // Merge multiple categories together (e.g. to count cached and uncached queries together)
  categoryRenames?: { [sourceDataName: string]: string };
  quantityType?: QuantityType;
  showCategoryTotals?: boolean;
}) {
  const chartData = useMemo(() => {
    const filledData = [];
    const dateSet = new Set(rows.map(({ ds }) => toNumericUTC(ds)));

    // Find the range of dates
    const minDate = Math.min(...Array.from(dateSet));
    const maxDate = Math.max(...Array.from(dateSet));

    // Fill in the missing dates
    for (let date = minDate; date <= maxDate; date += MS_IN_DAY) {
      const row = rows.find(({ ds }) => toNumericUTC(ds) === date);
      filledData.push({
        dateNumeric: date,
        ...(row
          ? Object.fromEntries(
              Object.entries(
                groupBy(row.metrics, (metric) => {
                  const tag = categoryRenames[metric.tag] ?? metric.tag;
                  if (!(tag in categories)) {
                    Sentry.captureMessage(`Unexpected call tag “${tag}”`);
                  }
                  return tag;
                }),
              ).map(([key, keyMetrics]) => [
                key,
                sumBy(keyMetrics, (metric) => metric.value),
              ]),
            )
          : Object.fromEntries(Object.keys(categories).map((key) => [key, 0]))),
      });
    }

    return filledData;
  }, [rows, categoryRenames, categories]);

  const totalByTag = useMemo(
    () =>
      Object.fromEntries(
        Object.entries(
          groupBy(
            rows.flatMap((row) => row.metrics),
            (metric) => categoryRenames[metric.tag] ?? metric.tag,
          ),
        ).map(([tag, entries]) => [
          tag,
          sumBy(entries, (entry) => entry.value),
        ]),
      ),
    [rows, categoryRenames],
  );

  if (!rows.some(({ metrics }) => metrics.some(({ value }) => value > 0))) {
    return <UsageNoDataError entity={entity} />;
  }

  return (
    <div className="h-56">
      <DailyChart
        data={chartData}
        quantityType={quantityType}
        showCategoryInTooltip
      >
        {Object.entries(categories).map(([tag, { name, color }]) => (
          <Bar
            key={tag}
            dataKey={tag}
            className={color}
            name={` ${name}`}
            barSize={chartData.length === 1 ? SINGLE_BAR_WIDTH : undefined}
            isAnimationActive={false}
            stackId="stack"
            shape={(props: any) => {
              // eslint-disable-next-line react/prop-types
              const { dateNumeric, name: categoryName } = props;
              if (
                typeof dateNumeric !== "number" ||
                typeof categoryName !== "string"
              ) {
                Sentry.captureMessage("Invalid props in stacked bar");
                return <Rectangle {...props} />;
              }

              return <Rectangle {...props} />;
            }}
          />
        ))}
        <Legend
          content={
            <div className="flex flex-wrap gap-3 pl-[72px]">
              {Object.entries(categories).map(([tag, { name, color }]) =>
                Object.hasOwn(totalByTag, tag) ? (
                  <span key={tag} className="mr-3 flex items-center gap-2">
                    <svg className="w-4" viewBox="0 0 50 50" aria-hidden>
                      <circle cx="25" cy="25" r="25" className={color} />
                    </svg>
                    <span>
                      <span>{name}</span>
                      {showCategoryTotals && (
                        <>: {formatQuantity(totalByTag[tag], quantityType)}</>
                      )}
                    </span>
                  </span>
                ) : null,
              )}
            </div>
          }
        />
      </DailyChart>
    </div>
  );
}

export function UsageBarChart({
  rows,
  entity,
  quantityType = "unit",
}: {
  rows: DailyMetric[];
  entity: string;
  quantityType?: QuantityType;
}) {
  // Sort rows and convert date to numeric values
  const chartData = useMemo(() => {
    const filledData = [];
    const dateSet = new Set(rows.map(({ ds }) => toNumericUTC(ds)));

    // Find the range of dates
    const minDate = Math.min(...Array.from(dateSet));
    const maxDate = Math.max(...Array.from(dateSet));

    // Fill in the missing dates
    for (let date = minDate; date <= maxDate; date += MS_IN_DAY) {
      const row = rows.find(({ ds }) => toNumericUTC(ds) === date);
      filledData.push({
        dateNumeric: date,
        value: row ? row.value : 0,
      });
    }

    return filledData;
  }, [rows]);

  if (!rows.some(({ value }) => value > 0)) {
    return <UsageNoDataError entity={entity} />;
  }

  return (
    <div className="h-48 animate-fadeInFromLoading">
      <DailyChart data={chartData} quantityType={quantityType}>
        <Bar
          dataKey="value"
          isAnimationActive={false}
          className="fill-chart-line-1"
          name={quantityType === "unit" ? entity : "bytes"}
          barSize={chartData.length === 1 ? SINGLE_BAR_WIDTH : undefined}
          minPointSize={4}
        />
      </DailyChart>
    </div>
  );
}

function DailyChart({
  data,
  showCategoryInTooltip = false,
  children,
  quantityType,
}: React.PropsWithChildren<{
  data: { dateNumeric: number }[];
  categoryInTooltip?: boolean;
  showCategoryInTooltip?: boolean;
  quantityType: QuantityType;
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
            width={60}
          />
          <Tooltip
            isAnimationActive={false}
            cursor={{
              fill: undefined, // Set in globals.css
            }}
            allowEscapeViewBox={{ y: true }}
            content={({ active, payload, label }) => (
              <ChartTooltip
                active={active}
                payload={payload?.map((dataPoint) => {
                  const prefix = showCategoryInTooltip
                    ? `${dataPoint.name}: `
                    : "";
                  const value = dataPoint.value as number;
                  const suffix =
                    !showCategoryInTooltip && quantityType === "unit"
                      ? ` ${dataPoint.name}`
                      : "";
                  return {
                    ...dataPoint,
                    formattedValue:
                      prefix + formatQuantity(value, quantityType) + suffix,
                  };
                })}
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
