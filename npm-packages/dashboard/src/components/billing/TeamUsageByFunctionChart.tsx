import {
  ChevronDownIcon,
  DesktopIcon,
  DownloadIcon,
} from "@radix-ui/react-icons";
import classNames from "classnames";
import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { useTeamEntitlements } from "api/teams";
import { AggregatedFunctionMetrics } from "hooks/usageMetrics";
import { rootComponentPath } from "api/usage";
import Link from "next/link";
import { useMemo, useState } from "react";
import {
  DeploymentResponse,
  DeploymentType,
  Team,
  ProjectDetails,
} from "generatedApi";
import { PuzzlePieceIcon } from "@common/elements/icons";
import { BANDWIDTH_CATEGORIES } from "./lib/teamUsageCategories";
import {
  QuantityType,
  formatQuantity,
  formatQuantityCompact,
} from "./lib/formatQuantity";

const ITEMS_SHOWN_INITIALLY = 6;

type DeploymentTypeRow = {
  key: string;
  function: string;
  componentPath: string;
  value: number;
  values: number[];
  deploymentType: DeploymentType | null;
  isSystem: boolean;
  isCloudBackups: boolean;
  href: string | null;
};

export type FunctionBreakdownMetric = {
  name: string;
  getTotal: (row: AggregatedFunctionMetrics) => number;
  getValues: (row: AggregatedFunctionMetrics) => number[];
  quantityType: QuantityType;
  categories?: {
    name: string;
    backgroundColor: string;
  }[];
};

export const FunctionBreakdownMetricCalls: FunctionBreakdownMetric = {
  name: "function calls",
  getTotal: (row) => row.callCount,
  getValues: (row) => [row.callCount],
  quantityType: "unit",
};

export const FunctionBreakdownMetricDatabaseBandwidth: FunctionBreakdownMetric =
  {
    name: "database bandwidth",
    getTotal: (row) => row.databaseIngressSize + row.databaseEgressSize,
    getValues: (row) => [row.databaseEgressSize, row.databaseIngressSize],
    quantityType: "storage",
    categories: Object.values(BANDWIDTH_CATEGORIES),
  };

export const FunctionBreakdownMetricActionCompute: FunctionBreakdownMetric = {
  name: "action compute",
  getTotal: (row) => row.actionComputeTime,
  getValues: (row) => [row.actionComputeTime],
  quantityType: "actionCompute",
};

export const FunctionBreakdownMetricVectorBandwidth: FunctionBreakdownMetric = {
  name: "vector bandwidth",
  getTotal: (row) => row.vectorIngressSize + row.vectorEgressSize,
  getValues: (row) => [row.vectorEgressSize, row.vectorIngressSize],
  quantityType: "storage",
  categories: Object.values(BANDWIDTH_CATEGORIES),
};

export function TeamUsageByFunctionChart({
  project,
  metric,
  deployments,
  rows,
  team,
  maxValue,
}: {
  project: ProjectDetails | null;
  metric: FunctionBreakdownMetric;
  deployments: DeploymentResponse[];
  rows: AggregatedFunctionMetrics[];
  team: Team;
  maxValue: number;
}) {
  const [showAll, setShowAll] = useState(false);

  const orderedAndGroupedRows = useOrderedAndGroupedRows(
    rows,
    metric,
    project,
    deployments,
    team,
  );

  const nonZeroRows = useMemo(
    () => orderedAndGroupedRows.filter((row) => row.value > 0),
    [orderedAndGroupedRows],
  );

  return (
    <div role="table">
      <div role="rowgroup" hidden aria-hidden={false}>
        <span role="columnheader">Function</span>
        <span role="columnheader">Deployment type</span>
        <span role="columnheader">Value</span>
      </div>

      <div className="relative" role="rowgroup">
        {nonZeroRows
          .slice(0, showAll ? undefined : ITEMS_SHOWN_INITIALLY)
          .map((row) => (
            <ChartRow
              key={row.key}
              row={row}
              maxValue={maxValue}
              quantityType={metric.quantityType}
              categories={metric.categories}
            />
          ))}

        {!showAll && nonZeroRows.length > ITEMS_SHOWN_INITIALLY && (
          <div className="h-4">
            <div className="bottom-four pointer-events-none absolute h-24 w-full bg-gradient-to-b from-transparent to-background-secondary" />
            <div className="absolute bottom-0 left-[50%]">
              <Button
                className="-translate-x-1/2"
                variant="neutral"
                onClick={() => setShowAll(true)}
                icon={<ChevronDownIcon />}
                inline
              >
                Show All
              </Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function ChartRow({
  row,
  maxValue,
  quantityType,
  categories,
}: {
  row: DeploymentTypeRow;
  maxValue: number;
  quantityType: QuantityType;
  categories:
    | {
        name: string;
        backgroundColor: string;
      }[]
    | undefined;
}) {
  const path = row.function;
  const { componentPath } = row;
  const isSystemFunction = row.isSystem;
  const { isCloudBackups } = row;
  const { module, functionName } = useMemo(() => {
    const separator = ".js:";
    const separatorPosition = path.indexOf(separator);

    if (isCloudBackups) {
      return { module: "Cloud Backup Generation", functionName: "default" };
    }

    if (isSystemFunction) {
      return { module: "Convex Dashboard", functionName: "default" };
    }

    if (separatorPosition === -1) {
      // HTTP Actions use the request path as the function name
      return { module: "HTTP", functionName: path };
    }

    return {
      module: path.substring(0, separatorPosition),
      functionName: path.substring(separatorPosition + separator.length),
    };
  }, [path, isSystemFunction, isCloudBackups]);

  const { values } = row;
  const nonZeroValues = values
    .map((value, i) => [value, i])
    .filter(([value]) => value > 0);
  const linkContents = (
    <div className="group relative flex h-10 py-1">
      <div role="cell" className="relative flex grow">
        <div className="absolute top-0 left-0 flex h-full w-full items-center">
          {nonZeroValues.map(([value, index], i) => (
            <div
              className={classNames(
                "flex h-6 min-w-[4px] items-center overflow-hidden",
                categories
                  ? categories[index].backgroundColor
                  : "bg-blue-200 dark:bg-cyan-900",
                i === 0 ? "rounded-l" : "",
                i === nonZeroValues.length - 1 ? "rounded-r" : "",
              )}
              key={index}
              style={{ width: `${(value / maxValue) * 100}%` }}
            />
          ))}
        </div>

        <div className="absolute top-0 left-0 flex h-full w-full items-center text-sm">
          <div className="truncate px-4">
            {isCloudBackups ? (
              <span className="flex items-center gap-1.5">
                <DownloadIcon />
                Cloud Backup Generation
              </span>
            ) : isSystemFunction ? (
              <span className="flex items-center gap-1.5">
                <DesktopIcon />
                Dashboard
              </span>
            ) : (
              <div className="flex items-center gap-1.5">
                {componentPath && componentPath !== rootComponentPath && (
                  <PuzzlePieceIcon className="min-w-[13px]" />
                )}
                <div>
                  {componentPath && componentPath !== rootComponentPath && (
                    <span className="font-mono text-content-secondary">
                      {componentPath}/
                    </span>
                  )}
                  <span className="font-mono font-semibold">{module}</span>
                  {functionName !== "default" && (
                    <span className="font-mono">
                      {module === "HTTP" ? " " : "."}
                      {functionName}
                    </span>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      <div role="cell" className="flex w-32 items-center pl-6">
        {row.deploymentType && (
          <DeploymentTypeIndicator deploymentType={row.deploymentType} />
        )}
      </div>

      <div
        role="cell"
        className="flex w-24 items-center justify-end px-4 whitespace-nowrap tabular-nums"
      >
        {formatQuantityCompact(row.value, quantityType)}
      </div>

      <div
        role="presentation"
        aria-hidden
        className={classNames(
          "absolute left-0 top-0 h-full w-full group-hover:bg-slate-900/5 dark:group-hover:bg-white/5 pointer-events-none rounded-sm",
        )}
      />
    </div>
  );

  const valueTip =
    categories !== undefined
      ? values.map((value, index) => (
          <div key={index}>
            <span
              className={classNames(
                "rounded-full w-2 h-2 inline-block",
                categories![index].backgroundColor,
              )}
            />{" "}
            {categories![index].name}: {formatQuantity(value, quantityType)}
          </div>
        ))
      : quantityType === "actionCompute"
        ? formatQuantity(values[0], quantityType)
        : null;

  const deploymentTypeTip =
    row.deploymentType === "dev" ? (
      <div>This row aggregates all dev deployments of your team.</div>
    ) : row.deploymentType === "preview" ? (
      <div>This row aggregates all preview deployments of your team.</div>
    ) : null;

  const systemFunctionTip = isSystemFunction ? (
    <div>
      Usage incurred by using the Convex dashboard, such as viewing the data or
      logs page for your deployment.
    </div>
  ) : null;
  const tip =
    valueTip !== null ||
    deploymentTypeTip !== null ||
    systemFunctionTip !== null ? (
      <div className="flex flex-col items-center gap-2">
        {valueTip !== null ? (
          <div className="flex flex-col items-end">{valueTip}</div>
        ) : null}
        {deploymentTypeTip}
        {systemFunctionTip}
      </div>
    ) : undefined;

  const rowContents = row.href ? (
    <Tooltip tip={tip} side="top" asChild>
      <Link passHref href={row.href}>
        {linkContents}
      </Link>
    </Tooltip>
  ) : (
    <Tooltip tip={tip} side="top" className="w-full">
      {linkContents}
    </Tooltip>
  );

  return <div role="row">{rowContents}</div>;
}

function DeploymentTypeIndicator({
  deploymentType,
}: {
  deploymentType: DeploymentType;
}) {
  switch (deploymentType) {
    case "prod":
      return (
        <>
          <span
            className={classNames(
              "w-4 h-4 rounded-xl mr-2 border bg-purple-100 dark:bg-purple-900",
            )}
          />
          <span className="capitalize">{deploymentType}</span>
        </>
      );
    case "dev":
      return (
        <>
          <span
            className={classNames(
              "w-4 h-4 rounded-xl mr-2 border bg-background-success",
            )}
          />
          <span className="capitalize">{deploymentType}</span>
        </>
      );
    case "preview":
      return (
        <>
          <span
            className={classNames(
              "w-4 h-4 rounded-xl mr-2 border bg-orange-100 dark:bg-orange-900",
            )}
          />
          <span className="capitalize">Preview</span>
        </>
      );
    default: {
      deploymentType satisfies never;
      return null;
    }
  }
}

/**
 * Groups the rows so that multiple development deployments are grouped together
 * and they are sorted by call count.
 */
function useOrderedAndGroupedRows(
  rows: AggregatedFunctionMetrics[],
  metric: FunctionBreakdownMetric,
  project: ProjectDetails | null,
  deployments: DeploymentResponse[],
  team: Team,
): DeploymentTypeRow[] {
  const arePreviewDeploymentsAvailable =
    useTeamEntitlements(team.id)?.projectMaxPreviewDeployments !== 0;
  // We should know about all active deployments in a project, including teammate's dev deployments.
  // When the project exists but we couldn't find the deployment, it means that it is a deactivated deployment.
  // If preview deployments are enabled, this is probably a preview deployment. But this could also be a dev
  // deployment for a teammate who left, so this fallback is imperfect.
  const fallbackDeploymentType: DeploymentType = arePreviewDeploymentsAvailable
    ? "preview"
    : "dev";
  return useMemo(() => {
    const byFunctionAndDeploymentType = rows.reduce(
      (accumulator, row) => {
        let deploymentType;
        const { componentPath } = row;
        let key;
        let deployment = null;
        const isSystem = row.function.startsWith("_system/");
        const isCloudBackups = row.function === "_system_job/cloud_backup";
        const name = isSystem ? "" : row.function;
        if (project) {
          deployment = deployments.find(
            (d) => d.id === row.deploymentId || d.name === row.deploymentName,
          );
          deploymentType = deployment
            ? deployment.deploymentType
            : fallbackDeploymentType;

          key = `${componentPath} ${name} ${deploymentType}`;
        } else {
          deploymentType = null;
          key = `${componentPath} ${name}`;
        }

        const total = metric.getTotal(row);
        const values = metric.getValues(row);
        if (key in accumulator) {
          const accumulated = accumulator[key];
          accumulated.value += total;
          values.forEach((value, index) => {
            accumulated.values[index] += value;
          });
        } else {
          accumulator[key] = {
            key,
            function: name,
            componentPath,
            value: total,
            values,
            deploymentType,
            isSystem,
            isCloudBackups,

            // We don’t link to development environments because they might belong to
            // someone else in the team. This might be improved later.
            href:
              project && deploymentType === "prod" && !isSystem
                ? `/t/${team.slug}/${project.slug}/${
                    deployment!.name
                  }/functions?function=${encodeURIComponent(
                    name.replace(".js", ""),
                  )}`
                : null,
          };
        }

        return accumulator;
      },
      {} as Record<string, DeploymentTypeRow>,
    );

    return Object.values(byFunctionAndDeploymentType).sort(
      (a, b) => b.value - a.value,
    );
  }, [rows, metric, deployments, project, team, fallbackDeploymentType]);
}
