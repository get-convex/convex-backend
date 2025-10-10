import {
  DailyMetricByProject,
  DailyPerTagMetricsByProject,
} from "hooks/usageMetrics";
import { useMemo } from "react";
import groupBy from "lodash/groupBy";
import sumBy from "lodash/sumBy";
import { ProjectDetails } from "generatedApi";
import { toNumericUTC } from "@common/lib/format";
import { Bar, Legend, Rectangle } from "recharts";
import { UsageNoDataError } from "./TeamUsageError";
import { QuantityType } from "./lib/formatQuantity";
import { DailyChart } from "./DailyChart";

// When there is only a data point, we have to set the bar width manually to make it appear (https://github.com/recharts/recharts/issues/3640).
// This value has been measured manually on a desktop screen size, but it should also look good in other contexts where there is only one bar.
const SINGLE_BAR_WIDTH = 91;

const MS_IN_DAY = 24 * 60 * 60 * 1000;

// Colors for projects - we'll cycle through these
const PROJECT_COLORS = [
  "fill-chart-line-1",
  "fill-chart-line-2",
  "fill-chart-line-3",
  "fill-chart-line-4",
  "fill-chart-line-5",
  "fill-chart-line-6",
  "fill-chart-line-7",
  "fill-chart-line-8",
];

function getProjectName(
  projectId: number | string,
  projects: ProjectDetails[] | undefined,
): string {
  if (projectId === "_rest") {
    return "All other projects";
  }
  const project = projects?.find((p) => p.id === projectId);
  return project?.name || `Deleted Project (${projectId})`;
}

export function UsageByProjectChart({
  rows,
  entity,
  quantityType = "unit",
  projects,
}: {
  rows: DailyPerTagMetricsByProject[] | DailyMetricByProject[];
  entity: string;
  quantityType?: QuantityType;
  projects: ProjectDetails[] | undefined;
}) {
  const { chartData, projectIds, legendProjectIds, totalByProject } =
    useMemo(() => {
      // Helper to get the total value from a row (handles both data types)
      const getRowTotal = (
        row: DailyPerTagMetricsByProject | DailyMetricByProject,
      ) => {
        if ("metrics" in row) {
          return sumBy(row.metrics, (m) => m.value);
        }
        return row.value;
      };

      // Get all unique project IDs and sort by total usage
      const byProject = groupBy(rows, (row) =>
        String(
          (row as DailyPerTagMetricsByProject | DailyMetricByProject).projectId,
        ),
      );
      const projectTotals = Object.entries(byProject).map(
        ([projectId, projectRows]) => {
          const parsedId = projectId === "_rest" ? "_rest" : Number(projectId);
          return {
            projectId: parsedId,
            total: sumBy(
              projectRows as Array<
                DailyPerTagMetricsByProject | DailyMetricByProject
              >,
              getRowTotal,
            ),
          };
        },
      );

      // Create quantity-sorted list for stacking (largest at bottom)
      const stackProjectIds = [...projectTotals]
        .sort((a, b) => {
          if (a.projectId === "_rest") return 1;
          if (b.projectId === "_rest") return -1;
          return b.total - a.total;
        })
        .map((p) => p.projectId);

      // Create alphabetically sorted list for legend display
      const alphabeticalProjectIds = [...projectTotals]
        .sort((a, b) => {
          if (a.projectId === "_rest") return 1;
          if (b.projectId === "_rest") return -1;
          const nameA = getProjectName(a.projectId, projects);
          const nameB = getProjectName(b.projectId, projects);
          return nameA.localeCompare(nameB);
        })
        .map((p) => p.projectId);

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

        // For each project, sum up all values for that day
        for (const projectId of stackProjectIds) {
          const projectRows = dayRows.filter((r) => r.projectId === projectId);
          const total = sumBy(projectRows, getRowTotal);
          dataPoint[`project_${projectId}`] = total;
        }

        filledData.push(dataPoint);
      }

      const totals = Object.fromEntries(
        projectTotals.map((p) => [p.projectId, p.total]),
      );

      return {
        chartData: filledData,
        projectIds: stackProjectIds,
        legendProjectIds: alphabeticalProjectIds,
        totalByProject: totals,
      };
    }, [rows, projects]);

  const colorMap = useMemo(() => {
    const map = new Map<string, string>();
    projectIds.forEach((projectId, index) => {
      const color = PROJECT_COLORS[index % PROJECT_COLORS.length];
      map.set(`project_${projectId}`, color);
    });
    return map;
  }, [projectIds]);

  // Compute display names with slugs for duplicates
  const displayNames = useMemo(() => {
    const nameCountMap = new Map<string, number>();
    projectIds.forEach((projectId) => {
      const name = getProjectName(projectId, projects);
      nameCountMap.set(name, (nameCountMap.get(name) || 0) + 1);
    });

    const names = new Map<
      number | string,
      { fullName: string; name: string; slug?: string }
    >();
    projectIds.forEach((projectId) => {
      const projectName = getProjectName(projectId, projects);
      const isDuplicate = (nameCountMap.get(projectName) || 0) > 1;

      let displayInfo = {
        fullName: projectName,
        name: projectName,
        slug: undefined as string | undefined,
      };
      if (isDuplicate && projectId !== "_rest") {
        const project = projects?.find((p) => p.id === projectId);
        if (project?.slug) {
          displayInfo = {
            fullName: `${projectName} (${project.slug})`,
            name: projectName,
            slug: project.slug,
          };
        }
      }
      names.set(projectId, displayInfo);
    });
    return names;
  }, [projectIds, projects]);

  if (
    !rows.some((row) => {
      if ("metrics" in row) {
        return row.metrics.some(({ value }) => value > 0);
      }
      return row.value > 0;
    })
  ) {
    return <UsageNoDataError entity={entity} />;
  }

  return (
    <div className="h-56">
      <DailyChart
        data={chartData}
        quantityType={quantityType}
        showCategoryInTooltip
        colorMap={colorMap}
        yAxisWidth={quantityType === "actionCompute" ? 80 : 60}
      >
        {projectIds.map((projectId, index) => {
          const displayInfo = displayNames.get(projectId);
          const color = PROJECT_COLORS[index % PROJECT_COLORS.length];

          return (
            <Bar
              key={projectId}
              dataKey={`project_${projectId}`}
              className={color}
              name={` ${displayInfo?.fullName || ""}`}
              barSize={chartData.length === 1 ? SINGLE_BAR_WIDTH : undefined}
              isAnimationActive={false}
              stackId="stack"
              shape={(props: any) => <Rectangle {...props} />}
            />
          );
        })}
        <Legend
          content={() => (
            <div
              className="scrollbar flex max-h-20 flex-wrap gap-3 overflow-y-auto"
              style={{
                paddingLeft: `${quantityType === "actionCompute" ? 92 : 72}px`,
              }}
            >
              {legendProjectIds.map((projectId) => {
                const displayInfo = displayNames.get(projectId);
                const stackIndex = projectIds.indexOf(projectId);
                const color =
                  PROJECT_COLORS[stackIndex % PROJECT_COLORS.length];
                const total = totalByProject[projectId] || 0;

                return total > 0 ? (
                  <span key={projectId} className="flex items-center gap-2">
                    <svg
                      className="w-4 flex-shrink-0"
                      viewBox="0 0 50 50"
                      aria-hidden
                    >
                      <circle cx="25" cy="25" r="25" className={color} />
                    </svg>
                    <span className="max-w-80 truncate">
                      {displayInfo?.name}
                      {displayInfo?.slug && (
                        <span className="text-content-secondary">
                          {" "}
                          ({displayInfo.slug})
                        </span>
                      )}
                    </span>
                  </span>
                ) : null;
              })}
            </div>
          )}
        />
      </DailyChart>
    </div>
  );
}
