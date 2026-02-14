import { DailyMetricByTable } from "hooks/usageMetrics";
import { useMemo } from "react";
import groupBy from "lodash/groupBy";
import sumBy from "lodash/sumBy";
import { toNumericUTC } from "@common/lib/format";
import { Bar, Legend, Rectangle } from "recharts";
import { useProjectById } from "api/projects";
import { UsageNoDataError } from "./TeamUsageError";
import { QuantityType, formatQuantity } from "./lib/formatQuantity";
import { DailyChart } from "./DailyChart";
import { DailyChartDetailView } from "./DailyChartDetailView";

// When there is only a data point, we have to set the bar width manually to make it appear (https://github.com/recharts/recharts/issues/3640).
// This value has been measured manually on a desktop screen size, but it should also look good in other contexts where there is only one bar.
const SINGLE_BAR_WIDTH = 91;

const MS_IN_DAY = 24 * 60 * 60 * 1000;

// Colors for tables - we'll cycle through these
const TABLE_COLORS = [
  "fill-chart-line-1",
  "fill-chart-line-2",
  "fill-chart-line-3",
  "fill-chart-line-4",
  "fill-chart-line-5",
  "fill-chart-line-6",
  "fill-chart-line-7",
  "fill-chart-line-8",
];

// Component to render "ProjectName / tableName"
function TableDisplayName({
  projectId,
  tableName,
}: {
  projectId: number | "_rest";
  tableName: string;
}) {
  const { project, isLoading } = useProjectById(
    projectId === "_rest" ? undefined : projectId,
  );

  // Handle the _rest category (all other tables)
  if (projectId === "_rest" && tableName === "_rest") {
    return <>All other tables</>;
  }

  if (isLoading) {
    return (
      <span className="inline-block h-4 w-32 animate-pulse rounded bg-content-tertiary" />
    );
  }

  const projectName = project?.name ? (
    project.name
  ) : (
    <span className="text-content-secondary">
      Deleted Project ({projectId})
    </span>
  );

  return (
    <>
      {projectName} / {tableName}
    </>
  );
}

// Custom tooltip component that renders table names
function TableTooltipItem({
  projectId,
  tableName,
  value,
  color,
  quantityType,
}: {
  projectId: number | "_rest";
  tableName: string;
  value: number;
  color: string;
  quantityType: QuantityType;
}) {
  return (
    <div className="flex items-center gap-2">
      <svg className="size-3 flex-shrink-0" viewBox="0 0 50 50" aria-hidden>
        <circle cx="25" cy="25" r="25" className={color} />
      </svg>
      <span className="tabular-nums">
        <TableDisplayName projectId={projectId} tableName={tableName} />:{" "}
        {formatQuantity(value, quantityType)}
      </span>
    </div>
  );
}

// Custom tooltip that can render table names
function TableChartTooltip({
  active,
  payload,
  label,
  quantityType,
  colorMap,
}: {
  active?: boolean;
  payload?: readonly any[];
  label?: any;
  quantityType: QuantityType;
  colorMap: Map<string, string>;
}) {
  if (!active || !payload || payload.length === 0) {
    return null;
  }

  // Filter to items with value > 0 and extract table names
  const items = payload
    .filter((entry) => {
      const value = entry.value as number;
      return value > 0;
    })
    .reverse(); // Reverse to show highest value first

  if (items.length === 0) {
    return null;
  }

  const formattedDate = new Date(label).toLocaleDateString("en-us", {
    year: "numeric",
    month: "long",
    day: "numeric",
    timeZone: "UTC",
  });

  return (
    <div className="rounded-sm border bg-background-secondary/70 p-3 backdrop-blur-[2px]">
      <div className="mb-2 font-semibold">{formattedDate}</div>
      <div className="space-y-1">
        {items.map((entry, index) => {
          // Extract project ID and table name from dataKey (format: "table_projectId_tableName")
          const dataKey = entry.dataKey as string;
          const projectTableStr = dataKey.replace("table_", "");
          const parts = projectTableStr.split("_");
          const projectId = projectTableStr.startsWith("_rest")
            ? "_rest"
            : Number(parts[0]);
          const tableName = projectTableStr.startsWith("_rest")
            ? "_rest"
            : parts.slice(1).join("_");
          const color = colorMap.get(entry.dataKey as string) || "";

          return (
            <TableTooltipItem
              key={index}
              projectId={projectId}
              tableName={tableName}
              value={entry.value as number}
              color={color}
              quantityType={quantityType}
            />
          );
        })}
      </div>
    </div>
  );
}

// Component for rendering a single table legend item
function TableLegendItem({
  projectId,
  tableName,
  color,
  total,
}: {
  projectId: number | "_rest";
  tableName: string;
  color: string;
  total: number;
}) {
  if (total <= 0) return null;

  return (
    <span className="flex items-center gap-2">
      <svg className="w-4 flex-shrink-0" viewBox="0 0 50 50" aria-hidden>
        <circle cx="25" cy="25" r="25" className={color} />
      </svg>
      <span className="max-w-80 truncate">
        <TableDisplayName projectId={projectId} tableName={tableName} />
      </span>
    </span>
  );
}

// Detail item that includes project ID and table name
interface TableDetailItem {
  projectId: number | "_rest";
  tableName: string;
  value: number;
  color: string;
}

// Component wrapper to render detail view with table names including project names
function TableChartDetailView({
  date,
  items,
  quantityType,
  onBack,
}: {
  date: number;
  items: TableDetailItem[];
  quantityType: QuantityType;
  onBack: () => void;
}) {
  // Convert TableDetailItem[] to the format expected by DailyChartDetailView
  const detailItems = items.map((item) => {
    // eslint-disable-next-line react-hooks/rules-of-hooks
    const { project } = useProjectById(
      item.projectId === "_rest" ? undefined : item.projectId,
    );

    let displayName: string;
    if (item.projectId === "_rest" && item.tableName === "_rest") {
      displayName = "All other tables";
    } else if (item.projectId === "_rest") {
      displayName = item.tableName;
    } else {
      const projectName =
        project?.name || `Deleted Project (${item.projectId})`;
      displayName = `${projectName} / ${item.tableName}`;
    }

    return {
      name: displayName,
      value: item.value,
      color: item.color,
    };
  });

  return (
    <DailyChartDetailView
      date={date}
      items={detailItems}
      quantityType={quantityType}
      onBack={onBack}
    />
  );
}

export function UsageByTableChart({
  rows,
  quantityType = "unit",
  selectedDate,
  setSelectedDate,
}: {
  rows: DailyMetricByTable[];
  quantityType?: QuantityType;
  selectedDate: number | null;
  setSelectedDate: (date: number | null) => void;
}) {
  const { chartData, tableKeys, totalByTable } = useMemo(() => {
    // Get all unique (projectId, tableName) combinations and sort by total usage
    // Use composite key: projectId_tableName
    const byTable = groupBy(rows, (row) => `${row.projectId}_${row.tableName}`);
    const tableTotals = Object.entries(byTable).map(([key, tableRows]) => {
      const firstRow = tableRows[0];
      return {
        key,
        projectId: firstRow.projectId,
        tableName: firstRow.tableName,
        total: sumBy(tableRows, (r) => r.value),
      };
    });

    // Create quantity-sorted list for stacking (largest at bottom)
    // Also used for legend display (sorted by quantity, not alphabetically)
    // "_rest" is always shown at the top of the stack
    const stackTableKeys = [...tableTotals]
      .sort((a, b) => {
        // Always put _rest at the end (top of stack)
        if (a.projectId === "_rest" && a.tableName === "_rest") return 1;
        if (b.projectId === "_rest" && b.tableName === "_rest") return -1;
        return b.total - a.total;
      })
      .map((t) => ({
        key: t.key,
        projectId: t.projectId,
        tableName: t.tableName,
      }));

    const filledData = [];
    const dateSet = new Set(rows.map(({ ds }) => toNumericUTC(ds)));

    // Find the range of dates
    const minDate = Math.min(...Array.from(dateSet));
    const maxDate = Math.max(...Array.from(dateSet));

    // Fill in the missing dates
    for (let date = minDate; date <= maxDate; date += MS_IN_DAY) {
      const dayRows = rows.filter(({ ds }) => toNumericUTC(ds) === date);
      const dataPoint: any = {
        dateNumeric: date,
      };

      // For each (projectId, tableName) combination, sum up all values for that day
      for (const { key, projectId, tableName } of stackTableKeys) {
        const tableRows = dayRows.filter(
          (r) => r.projectId === projectId && r.tableName === tableName,
        );
        const total = sumBy(tableRows, (r) => r.value);
        dataPoint[`table_${key}`] = total;
      }

      filledData.push(dataPoint);
    }

    const totals = Object.fromEntries(tableTotals.map((t) => [t.key, t.total]));

    return {
      chartData: filledData,
      tableKeys: stackTableKeys,
      totalByTable: totals,
    };
  }, [rows]);

  const colorMap = useMemo(() => {
    const map = new Map<string, string>();
    tableKeys.forEach(({ key }, index) => {
      const color = TABLE_COLORS[index % TABLE_COLORS.length];
      map.set(`table_${key}`, color);
    });
    return map;
  }, [tableKeys]);

  // Get detail items for selected date
  const detailItems = useMemo((): TableDetailItem[] => {
    if (selectedDate === null) return [];

    const dataPoint = chartData.find((d) => d.dateNumeric === selectedDate);
    if (!dataPoint) return [];

    return tableKeys.map(({ key, projectId, tableName }, index) => {
      const color = TABLE_COLORS[index % TABLE_COLORS.length];
      return {
        projectId,
        tableName,
        value: (dataPoint[`table_${key}`] as number) || 0,
        color,
      };
    });
  }, [selectedDate, chartData, tableKeys]);

  if (!rows.some((row) => row.value > 0)) {
    return <UsageNoDataError />;
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
          colorMap={colorMap}
          yAxisWidth={quantityType === "actionCompute" ? 80 : 60}
          customTooltip={(props) => (
            <TableChartTooltip
              {...props}
              quantityType={quantityType}
              colorMap={colorMap}
            />
          )}
        >
          {tableKeys.map(({ key }, index) => {
            const color = TABLE_COLORS[index % TABLE_COLORS.length];

            return (
              <Bar
                key={key}
                dataKey={`table_${key}`}
                className={color}
                name={` `} // Space for consistent tooltip formatting
                barSize={chartData.length === 1 ? SINGLE_BAR_WIDTH : undefined}
                isAnimationActive={false}
                stackId="stack"
                style={{ cursor: "pointer" }}
                tabIndex={0}
                onClick={(data) => {
                  const cd = data as any;
                  if (cd?.dateNumeric) {
                    setSelectedDate(cd.dateNumeric);
                  }
                }}
                onKeyDown={(data, _idx, event) => {
                  if (event.key === "Enter") {
                    const cd = data as any;
                    if (cd?.dateNumeric) {
                      setSelectedDate(cd.dateNumeric);
                    }
                  }
                }}
                shape={(props: any) => <Rectangle {...props} />}
              />
            );
          })}
          {selectedDate === null && (
            <Legend
              content={() => (
                <div
                  className="scrollbar flex max-h-20 flex-wrap gap-3 overflow-y-auto"
                  style={{
                    paddingLeft: `${quantityType === "actionCompute" ? 92 : 72}px`,
                  }}
                >
                  {tableKeys.map(({ key, projectId, tableName }, index) => {
                    const color = TABLE_COLORS[index % TABLE_COLORS.length];
                    const total = totalByTable[key] || 0;

                    return (
                      <TableLegendItem
                        key={key}
                        projectId={projectId}
                        tableName={tableName}
                        color={color}
                        total={total}
                      />
                    );
                  })}
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
          <TableChartDetailView
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
