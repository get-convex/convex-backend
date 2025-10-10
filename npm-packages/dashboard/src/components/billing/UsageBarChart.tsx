import * as Sentry from "@sentry/nextjs";
import { DailyMetric, DailyPerTagMetrics } from "hooks/usageMetrics";
import { Bar, Legend, Rectangle } from "recharts";
import { useMemo } from "react";
import groupBy from "lodash/groupBy";
import sumBy from "lodash/sumBy";
import { toNumericUTC } from "@common/lib/format";
import { QuantityType, formatQuantity } from "./lib/formatQuantity";
import { UsageNoDataError } from "./TeamUsageError";
import { DailyChart } from "./DailyChart";
import {
  DailyChartDetailView,
  DailyChartDetailItem,
} from "./DailyChartDetailView";

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
  selectedDate,
  setSelectedDate,
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
  selectedDate: number | null;
  setSelectedDate: (date: number | null) => void;
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
                    Sentry.captureMessage(
                      `Unexpected call tag "${tag}"`,
                      "error",
                    );
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

  // Get detail items for selected date
  const detailItems = useMemo((): DailyChartDetailItem[] => {
    if (selectedDate === null) return [];

    const dataPoint = chartData.find((d) => d.dateNumeric === selectedDate);
    if (!dataPoint) return [];

    return Object.entries(categories).map(([tag, { name, color }]) => ({
      name,
      value: ((dataPoint as any)[tag] as number) || 0,
      color,
      project: undefined, // Category-based items don't have projects
    }));
  }, [selectedDate, chartData, categories]);

  if (!rows.some(({ metrics }) => metrics.some(({ value }) => value > 0))) {
    return <UsageNoDataError entity={entity} />;
  }

  return (
    <div
      className={`relative overflow-hidden transition-all duration-300 ${
        selectedDate !== null ? "h-[32rem]" : "h-56"
      }`}
    >
      {/* Background chart (slides out to left when detail view is shown) */}
      <div
        className="absolute inset-0 transition-transform duration-300 ease-in-out"
        style={{
          transform:
            selectedDate !== null ? "translateX(-100%)" : "translateX(0)",
        }}
      >
        <DailyChart
          data={chartData}
          quantityType={quantityType}
          showCategoryInTooltip
          yAxisWidth={quantityType === "actionCompute" ? 80 : 60}
        >
          {Object.entries(categories).map(([tag, { name, color }]) => (
            <Bar
              key={tag}
              dataKey={tag}
              className={color}
              name={name}
              barSize={chartData.length === 1 ? SINGLE_BAR_WIDTH : undefined}
              isAnimationActive={false}
              stackId="stack"
              style={{ cursor: "pointer" }}
              onClick={(data: any) => {
                if (data?.dateNumeric) {
                  setSelectedDate(data.dateNumeric);
                }
              }}
              shape={(props: any) => {
                // eslint-disable-next-line react/prop-types
                const { dateNumeric, name: categoryName } = props;
                if (
                  typeof dateNumeric !== "number" ||
                  typeof categoryName !== "string"
                ) {
                  Sentry.captureMessage(
                    "Invalid props in stacked bar",
                    "error",
                  );
                  return <Rectangle {...props} />;
                }

                return <Rectangle {...props} />;
              }}
            />
          ))}
          {selectedDate === null && (
            <Legend
              content={() => (
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
                            <>
                              : {formatQuantity(totalByTag[tag], quantityType)}
                            </>
                          )}
                        </span>
                      </span>
                    ) : null,
                  )}
                </div>
              )}
            />
          )}
        </DailyChart>
      </div>

      {/* Detail view (slides in from right) */}
      <div
        className="absolute inset-0 transition-transform duration-300 ease-in-out"
        style={{
          transform:
            selectedDate !== null ? "translateX(0)" : "translateX(100%)",
        }}
      >
        {selectedDate !== null && (
          <DailyChartDetailView
            date={selectedDate}
            items={detailItems}
            quantityType={quantityType}
            onBack={() => setSelectedDate(null)}
          />
        )}
      </div>
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
      <DailyChart
        data={chartData}
        quantityType={quantityType}
        yAxisWidth={quantityType === "actionCompute" ? 80 : 60}
      >
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
