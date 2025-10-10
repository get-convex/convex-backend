import {
  DailyMetricByProject,
  DailyPerTagMetricsByProject,
} from "hooks/usageMetrics";
import { useMemo } from "react";
import groupBy from "lodash/groupBy";
import sumBy from "lodash/sumBy";
import { ProjectDetails, Team } from "generatedApi";
import { toNumericUTC } from "@common/lib/format";
import { Bar, Legend, Rectangle } from "recharts";
import { useDeployments } from "api/deployments";
import { useProfile } from "api/profile";
import { UsageNoDataError } from "./TeamUsageError";
import { QuantityType } from "./lib/formatQuantity";
import { DailyChart } from "./DailyChart";
import {
  DailyChartDetailView,
  DailyChartDetailItem,
} from "./DailyChartDetailView";

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

// Hook to get deployment hrefs for each project
function useProjectDeploymentHrefs(
  projectIds: (number | string)[],
  projects: ProjectDetails[] | undefined,
  team: Team | undefined,
  memberId: number | undefined,
): Map<number | string, { href?: string; loading: boolean }> {
  // Fetch deployments for all projects
  const deploymentsData = projectIds.map((projectId) => {
    const project =
      projectId !== "_rest" ? projects?.find((p) => p.id === projectId) : null;
    // eslint-disable-next-line react-hooks/rules-of-hooks
    const { deployments } = useDeployments(project?.id);
    return { projectId, deployments, project };
  });

  return useMemo(() => {
    const map = new Map<number | string, { href?: string; loading: boolean }>();

    for (const { projectId, deployments, project } of deploymentsData) {
      if (projectId === "_rest" || !project || !team) {
        map.set(projectId, { loading: false });
        continue;
      }

      if (!deployments) {
        map.set(projectId, { loading: true });
        continue;
      }

      const prodDeployment = deployments.find(
        (d) => d.deploymentType === "prod",
      );
      const devDeployment = deployments.find(
        (d) => d.deploymentType === "dev" && d.creator === memberId,
      );
      const anyDeployment = deployments[0];
      const shownDeployment = devDeployment ?? prodDeployment ?? anyDeployment;

      if (shownDeployment) {
        map.set(projectId, {
          href: `/t/${team.slug}/${project.slug}/${shownDeployment.name}`,
          loading: false,
        });
      } else {
        map.set(projectId, { loading: false });
      }
    }

    return map;
  }, [deploymentsData, team, memberId]);
}

export function UsageByProjectChart({
  rows,
  entity,
  quantityType = "unit",
  projects,
  team,
  selectedDate,
  setSelectedDate,
}: {
  rows: DailyPerTagMetricsByProject[] | DailyMetricByProject[];
  entity: string;
  quantityType?: QuantityType;
  projects: ProjectDetails[] | undefined;
  team?: Team;
  selectedDate: number | null;
  setSelectedDate: (date: number | null) => void;
}) {
  const member = useProfile();

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

  // Get deployment hrefs for projects
  const _projectHrefs = useProjectDeploymentHrefs(
    projectIds,
    projects,
    team,
    member?.id,
  );

  // Get detail items for selected date
  const detailItems = useMemo((): DailyChartDetailItem[] => {
    if (selectedDate === null) return [];

    const dataPoint = chartData.find((d) => d.dateNumeric === selectedDate);
    if (!dataPoint) return [];

    return projectIds.map((projectId, index) => {
      const color = PROJECT_COLORS[index % PROJECT_COLORS.length];
      const project =
        projectId === "_rest"
          ? null
          : (projects?.find((p) => p.id === projectId) ?? null);
      return {
        project,
        value: (dataPoint[`project_${projectId}`] as number) || 0,
        color,
      };
    });
  }, [selectedDate, chartData, projectIds, projects]);

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
                style={{ cursor: "pointer" }}
                tabIndex={0}
                onClick={(data: any) => {
                  if (data?.dateNumeric) {
                    setSelectedDate(data.dateNumeric);
                  }
                }}
                onKeyDown={(data, _idx, event) => {
                  if (event.key === "Enter") {
                    if (data?.dateNumeric) {
                      setSelectedDate(data.dateNumeric);
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
            team={team}
            memberId={member?.id}
          />
        )}
      </div>
    </div>
  );
}
