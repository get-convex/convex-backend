import { PlanSummary, UsageOverview } from "components/billing/PlanSummary";
import {
  Sheet,
  Loading,
  sidebarLinkClassNames,
  Button,
  formatBytes,
  formatNumberCompact,
} from "dashboard-common";
import {
  AggregatedFunctionMetrics,
  DateRange,
  useCurrentBillingPeriod,
  useUsageTeamActionComputeDaily,
  useUsageTeamMetricsByFunction,
  useUsageTeamDailyCallsByTag,
  useUsageTeamDatabaseBandwidthPerDay,
  useUsageTeamDocumentsPerDay,
  useUsageTeamDatabaseStoragePerDay,
  useUsageTeamStoragePerDay,
  useUsageTeamStorageThroughputDaily,
  useUsageTeamVectorBandwidthPerDay,
  useUsageTeamVectorStoragePerDay,
  useUsageTeamSummary,
} from "hooks/usageMetrics";
import { Team, ProjectDetails } from "generatedApi";
import { ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { useDeployments } from "api/deployments";
import { useTeamEntitlements } from "api/teams";
import { useProjects } from "api/projects";
import { useTeamOrbSubscription } from "api/billing";
import Link from "next/link";
import groupBy from "lodash/groupBy";
import sumBy from "lodash/sumBy";
import { Tab } from "@headlessui/react";
import classNames from "classnames";
import { Period } from "elements/UsagePeriodSelector";
import { formatQuantity } from "data/Charts/formatQuantity";
import {
  DATABASE_STORAGE_CATEGORIES,
  BANDWIDTH_CATEGORIES,
  CATEGORY_RENAMES,
  TAG_CATEGORIES,
  FILE_BANDWIDTH_CATEGORIES,
  FILE_STORAGE_CATEGORIES,
} from "data/Charts/teamUsageCategories";
import { useRouter } from "next/router";
import { ExternalLinkIcon } from "@radix-ui/react-icons";
import { FunctionBreakdownSelector } from "./FunctionBreakdownSelector";
import {
  FunctionBreakdownMetric,
  FunctionBreakdownMetricActionCompute,
  FunctionBreakdownMetricCalls,
  FunctionBreakdownMetricDatabaseBandwidth,
  FunctionBreakdownMetricVectorBandwidth,
  TeamUsageByFunctionChart,
} from "./TeamUsageByFunctionChart";
import { UsageBarChart, UsageStackedBarChart } from "./UsageBarChart";
import {
  UsageChartUnavailable,
  UsageDataNotAvailable,
  UsageNoDataError,
} from "./TeamUsageError";
import { TeamUsageToolbar } from "./TeamUsageToolbar";

const FUNCTION_BREAKDOWN_TABS = [
  FunctionBreakdownMetricCalls,
  FunctionBreakdownMetricDatabaseBandwidth,
  FunctionBreakdownMetricActionCompute,
  FunctionBreakdownMetricVectorBandwidth,
];

export function TeamUsage({ team }: { team: Team }) {
  const projects = useProjects(team.id);
  const { query } = useRouter();
  const project = query.projectSlug
    ? projects?.find((p) => p.slug === query.projectSlug)
    : null;
  const projectId = project?.id ?? null;

  const componentPrefix = (query.componentPrefix ?? null) as string | null;

  const [selectedBillingPeriod, setSelectedBillingPeriod] =
    useState<Period | null>(null);
  const currentBillingPeriod = useCurrentBillingPeriod(team.id);
  const shownBillingPeriod =
    selectedBillingPeriod === null && currentBillingPeriod !== undefined
      ? ({
          type: "currentBillingPeriod",
          from: currentBillingPeriod.start,
          to: currentBillingPeriod.end,
        } as const)
      : selectedBillingPeriod;

  const dateRange =
    shownBillingPeriod !== null &&
    shownBillingPeriod.type !== "currentBillingPeriod"
      ? { from: shownBillingPeriod.from, to: shownBillingPeriod.to }
      : null;

  const metricsByFunction = useUsageTeamMetricsByFunction(
    team.id,
    dateRange,
    projectId,
    componentPrefix,
  );

  const [functionBreakdownTabIndex, setFunctionBreakdownTabIndex] = useState(0);

  const isFunctionBreakdownBandwidthAvailable =
    shownBillingPeriod === null || shownBillingPeriod.from >= "2024-01-01";

  const { subscription } = useTeamOrbSubscription(team?.id);

  const teamSummary = useUsageTeamSummary(
    team?.id,
    shownBillingPeriod
      ? { from: shownBillingPeriod.from, to: shownBillingPeriod.to }
      : null,
    projectId,
    componentPrefix,
  );

  const entitlements = useTeamEntitlements(team?.id);

  const hasOrbSubscription = useHasSubscription(team?.id);

  const hasSubscription =
    (!shownBillingPeriod ||
      shownBillingPeriod.type === "currentBillingPeriod") &&
    (hasOrbSubscription || hasOrbSubscription === undefined) &&
    projectId === null;

  const showEntitlements =
    (!shownBillingPeriod ||
      shownBillingPeriod.type === "currentBillingPeriod") &&
    projectId === null;

  return (
    <div>
      <div className="flex justify-between">
        <h2>Usage</h2>
        {subscription !== undefined && (
          <Button
            href={`/t/${team?.slug}/settings/billing`}
            variant="neutral"
            icon={<ExternalLinkIcon />}
            className="animate-fadeInFromLoading"
            size="xs"
          >
            {subscription
              ? "View Subscription & Invoices"
              : "Upgrade Subscription"}
          </Button>
        )}
      </div>

      {currentBillingPeriod !== undefined &&
        shownBillingPeriod !== null &&
        projects && (
          <>
            <TeamUsageToolbar
              {...{
                shownBillingPeriod,
                setSelectedBillingPeriod,
                currentBillingPeriod,
                projects,
                projectId,
              }}
            />

            <div className="mb-6 flex flex-col gap-6">
              <PlanSummary
                teamSummary={teamSummary}
                entitlements={entitlements}
                hasSubscription={hasSubscription}
                showEntitlements={showEntitlements}
              />

              <FunctionCallsUsage
                team={team}
                dateRange={dateRange}
                projectId={projectId}
                componentPrefix={componentPrefix}
                functionCalls={teamSummary?.functionCalls}
                functionCallsEntitlement={entitlements?.teamMaxFunctionCalls}
                showEntitlements={showEntitlements}
              />

              <ActionComputeUsage
                team={team}
                dateRange={dateRange}
                projectId={projectId}
                componentPrefix={componentPrefix}
                actionCompute={teamSummary?.actionCompute}
                actionComputeEntitlement={entitlements?.teamMaxActionCompute}
                showEntitlements={showEntitlements}
              />

              <DatabaseUsage
                team={team}
                dateRange={dateRange}
                projectId={projectId}
                componentPrefix={componentPrefix}
                storage={teamSummary?.databaseStorage}
                storageEntitlement={entitlements?.teamMaxDatabaseStorage}
                bandwidth={teamSummary?.databaseBandwidth}
                bandwidthEntitlement={entitlements?.teamMaxDatabaseBandwidth}
                showEntitlements={showEntitlements}
              />

              <FilesUsage
                team={team}
                dateRange={dateRange}
                projectId={projectId}
                componentPrefix={componentPrefix}
                storage={teamSummary?.fileStorage}
                storageEntitlement={entitlements?.teamMaxFileStorage}
                bandwidth={teamSummary?.fileBandwidth}
                bandwidthEntitlement={entitlements?.teamMaxFileBandwidth}
                showEntitlements={showEntitlements}
              />

              <VectorUsage
                team={team}
                dateRange={dateRange}
                projectId={projectId}
                componentPrefix={componentPrefix}
                storage={teamSummary?.vectorStorage}
                storageEntitlement={entitlements?.teamMaxVectorStorage}
                bandwidth={teamSummary?.vectorBandwidth}
                bandwidthEntitlement={entitlements?.teamMaxVectorBandwidth}
                showEntitlements={showEntitlements}
              />

              <Sheet padding={false} className="pb-4">
                <div className="mb-4 flex w-full flex-wrap items-center justify-between gap-4 border-b p-4 py-2">
                  <h3>Functions breakdown by project</h3>

                  <FunctionBreakdownSelector
                    value={functionBreakdownTabIndex}
                    onChange={setFunctionBreakdownTabIndex}
                  />
                </div>
                <div className="px-4">
                  {!metricsByFunction || !projects ? (
                    <ChartLoading />
                  ) : functionBreakdownTabIndex === 0 ||
                    isFunctionBreakdownBandwidthAvailable ? (
                    <FunctionUsageBreakdown
                      team={team}
                      projects={projects}
                      metricsByDeployment={metricsByFunction}
                      metric={
                        FUNCTION_BREAKDOWN_TABS[functionBreakdownTabIndex]
                      }
                    />
                  ) : (
                    <UsageDataNotAvailable
                      entity={`Breakdown by ${FUNCTION_BREAKDOWN_TABS[functionBreakdownTabIndex].name}`}
                    />
                  )}
                </div>
              </Sheet>
            </div>
          </>
        )}
    </div>
  );
}

function useUsageByProject(
  callsByDeployment: AggregatedFunctionMetrics[],
  projects: ProjectDetails[],
  metric: FunctionBreakdownMetric,
): {
  key: string;
  project: ProjectDetails | null;
  rows: AggregatedFunctionMetrics[];
  total: number;
}[] {
  return useMemo(() => {
    const byProject = groupBy(callsByDeployment, (row) => row.projectId);
    return Object.entries(byProject)
      .map(([projectId, rows]) => ({
        key: projectId,
        project: projects.find((p) => p.id === rows[0].projectId) ?? null,
        rows,
        total: sumBy(rows, metric.getTotal),
      }))
      .sort((a, b) => b.total - a.total);
  }, [projects, callsByDeployment, metric]);
}

function ChartLoading() {
  return <Loading className="h-56 w-full rounded" fullHeight={false} />;
}

function FunctionUsageBreakdown({
  projects,
  team,
  metricsByDeployment,
  metric,
}: {
  projects: ProjectDetails[];
  metricsByDeployment: AggregatedFunctionMetrics[];
  metric: FunctionBreakdownMetric;
  team: Team;
}) {
  const usageByProject = useUsageByProject(
    metricsByDeployment,
    projects,
    metric,
  );

  if (usageByProject.length === 0) {
    return <UsageNoDataError entity={metric.name} />;
  }

  const maxValue = Math.max(...metricsByDeployment.map(metric.getTotal));

  if (maxValue === 0) {
    return <UsageNoDataError entity={metric.name} />;
  }

  return (
    <div className="animate-fadeInFromLoading overflow-y-auto scrollbar">
      {usageByProject.map(({ key, project, rows, total }) => (
        <FunctionUsageBreakdownByProject
          key={key}
          project={project}
          metric={metric}
          rows={rows}
          projectTotal={total}
          maxValue={maxValue}
          team={team}
        />
      ))}
      {metric.categories !== undefined ? (
        <div className="flex items-center gap-6">
          {metric.categories.map((category, index) => (
            <div key={index} className="flex items-center gap-2">
              <div
                className={classNames(
                  "w-4 h-4 rounded-full",
                  category.backgroundColor,
                )}
              />
              <span className="text-xs font-medium">{category.name}</span>
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function FunctionUsageBreakdownByProject({
  project,
  metric,
  rows,
  maxValue,
  team,
  projectTotal,
}: {
  project: ProjectDetails | null;
  metric: FunctionBreakdownMetric;
  rows: AggregatedFunctionMetrics[];
  team: Team;
  maxValue: number;
  projectTotal: number;
}) {
  const { deployments } = useDeployments(project?.id);
  const isLoadingDeployments = project && !deployments;

  if (projectTotal === 0) {
    return null;
  }

  return (
    <div className="mb-4">
      <p className="flex align-baseline font-medium">
        {project && (
          <Link
            href={`/t/${team.slug}/${project.slug}/`}
            passHref
            className="inline-flex items-baseline gap-2 py-2"
          >
            <span>{project.name}</span>
            {project.name?.toLowerCase() !== project.slug ? (
              <span className="text-sm text-content-secondary">
                {project.slug}
              </span>
            ) : null}
          </Link>
        )}
        {!project && (
          <span className="inline-block py-2 italic">Deleted Project</span>
        )}
        <span className="flex-1 px-4 py-2 text-right tabular-nums">
          {formatQuantity(projectTotal, metric.quantityType)}
        </span>
      </p>

      {isLoadingDeployments && <ChartLoading />}
      {!isLoadingDeployments && (
        <TeamUsageByFunctionChart
          project={project}
          deployments={deployments ?? []}
          metric={metric}
          rows={rows}
          team={team}
          maxValue={maxValue}
        />
      )}
    </div>
  );
}

function UsageTab({ children }: { children: ReactNode }) {
  return (
    <Tab
      className={({ selected }) =>
        sidebarLinkClassNames({
          isActive: selected,
        })
      }
    >
      {children}
    </Tab>
  );
}

function DatabaseUsage({
  team,
  dateRange,
  projectId,
  storage,
  bandwidth,
  storageEntitlement,
  bandwidthEntitlement,
  showEntitlements,
  componentPrefix,
}: {
  team: Team;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
  storage?: number;
  bandwidth?: number;
  storageEntitlement?: number | null;
  bandwidthEntitlement?: number | null;
  showEntitlements: boolean;
}) {
  const databaseStorage = useUsageTeamDatabaseStoragePerDay(
    team.id,
    projectId,
    dateRange,
    componentPrefix,
  );

  const documentsCount = useUsageTeamDocumentsPerDay(
    team.id,
    projectId,
    dateRange,
    componentPrefix,
  );

  const databaseBandwidth = useUsageTeamDatabaseBandwidthPerDay(
    team.id,
    projectId,
    dateRange,
    componentPrefix,
  );

  const router = useRouter();
  const ref = useRef<HTMLDivElement>(null);

  const [selectedTab, setSelectedTab] = useState(0);

  useEffect(() => {
    const onHashChangeStart = (url: string) => {
      const hash = url.split("#")[1];
      if (hash === "databaseStorage" || hash === "databaseBandwidth") {
        ref.current?.scrollIntoView({
          behavior: "smooth",
          block: "start",
        });
      }
      if (hash === "databaseStorage") {
        setSelectedTab(0);
      }
      if (hash === "databaseBandwidth") {
        setSelectedTab(1);
      }
    };

    router.events.on("hashChangeStart", onHashChangeStart);

    return () => {
      router.events.off("hashChangeStart", onHashChangeStart);
    };
  }, [router.events]);

  return (
    <Sheet ref={ref} padding={false} className="pb-4">
      <Tab.Group selectedIndex={selectedTab} onChange={setSelectedTab}>
        <div className="relative mb-4 flex w-full flex-wrap items-center justify-between gap-4 border-b p-4 py-2">
          <h3>Database</h3>

          <Tab.List className="flex gap-2">
            <UsageTab>Storage</UsageTab>
            <UsageTab>Bandwidth</UsageTab>
            <UsageTab>Document Count</UsageTab>
          </Tab.List>
        </div>
        <Tab.Panels className="px-4">
          <Tab.Panel>
            {showEntitlements && (
              <UsageOverview
                metric={storage}
                entitlement={storageEntitlement ?? 0}
                format={formatBytes}
                showEntitlements={showEntitlements}
              />
            )}
            {databaseStorage === undefined ? (
              <ChartLoading />
            ) : databaseStorage === null ? (
              <UsageChartUnavailable />
            ) : (
              <UsageStackedBarChart
                rows={databaseStorage}
                categories={DATABASE_STORAGE_CATEGORIES}
                entity="documents"
                quantityType="storage"
                showCategoryTotals={false}
              />
            )}
          </Tab.Panel>
          <Tab.Panel>
            {showEntitlements && (
              <UsageOverview
                metric={bandwidth}
                entitlement={bandwidthEntitlement ?? 0}
                format={formatBytes}
                showEntitlements={showEntitlements}
              />
            )}
            {databaseBandwidth === undefined ? (
              <ChartLoading />
            ) : (
              <UsageStackedBarChart
                rows={databaseBandwidth}
                categories={BANDWIDTH_CATEGORIES}
                entity="documents"
                quantityType="storage"
              />
            )}
          </Tab.Panel>
          <Tab.Panel>
            {documentsCount === undefined ? (
              <ChartLoading />
            ) : documentsCount === null ? (
              <UsageChartUnavailable />
            ) : (
              <UsageBarChart rows={documentsCount} entity="documents" />
            )}
          </Tab.Panel>
        </Tab.Panels>
      </Tab.Group>
    </Sheet>
  );
}

function FunctionCallsUsage({
  team,
  dateRange,
  projectId,
  componentPrefix,
  functionCalls,
  functionCallsEntitlement,
  showEntitlements,
}: {
  team: Team;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
  functionCalls?: number;
  functionCallsEntitlement?: number | null;
  showEntitlements: boolean;
}) {
  const callsByTag = useUsageTeamDailyCallsByTag(
    team.id,
    projectId,
    dateRange,
    componentPrefix,
  );
  const router = useRouter();
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onHashChangeStart = (url: string) => {
      const hash = url.split("#")[1];
      if (hash === "functionCalls") {
        ref.current?.scrollIntoView({
          behavior: "smooth",
          block: "start",
        });
      }
    };

    router.events.on("hashChangeStart", onHashChangeStart);

    return () => {
      router.events.off("hashChangeStart", onHashChangeStart);
    };
  }, [router.events]);

  return (
    <Sheet ref={ref} padding={false} className="pb-4">
      <div className="relative mb-4 flex w-full flex-wrap items-center justify-between gap-4 border-b p-4">
        <h3>Daily function calls</h3>
      </div>
      <div className="px-4">
        {showEntitlements && (
          <UsageOverview
            metric={functionCalls}
            entitlement={functionCallsEntitlement ?? 0}
            format={formatNumberCompact}
            showEntitlements={showEntitlements}
          />
        )}
        {callsByTag === undefined ? (
          <ChartLoading />
        ) : (
          <UsageStackedBarChart
            rows={callsByTag}
            entity="calls"
            categories={TAG_CATEGORIES}
            categoryRenames={CATEGORY_RENAMES}
          />
        )}
      </div>
    </Sheet>
  );
}

function ActionComputeUsage({
  team,
  dateRange,
  projectId,
  componentPrefix,
  actionCompute,
  actionComputeEntitlement,
  showEntitlements,
}: {
  team: Team;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
  actionCompute?: number;
  actionComputeEntitlement?: number | null;
  showEntitlements: boolean;
}) {
  const actionComputeDaily = useUsageTeamActionComputeDaily(
    team.id,
    projectId,
    dateRange,
    componentPrefix,
  );

  const router = useRouter();
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onHashChangeStart = (url: string) => {
      const hash = url.split("#")[1];
      if (hash === "actionCompute") {
        ref.current?.scrollIntoView({
          behavior: "smooth",
          block: "start",
        });
      }
    };

    router.events.on("hashChangeStart", onHashChangeStart);

    return () => {
      router.events.off("hashChangeStart", onHashChangeStart);
    };
  }, [router.events]);

  return (
    <Sheet ref={ref} padding={false} className="pb-4">
      <div className="relative mb-4 flex w-full flex-wrap items-center justify-between gap-4 border-b p-4">
        <h3>Action Compute</h3>
      </div>
      <div className="px-4">
        {showEntitlements && (
          <UsageOverview
            metric={actionCompute}
            entitlement={actionComputeEntitlement ?? 0}
            format={formatNumberCompact}
            showEntitlements={showEntitlements}
            suffix="GB-hours"
          />
        )}
        {actionComputeDaily === undefined ? (
          <ChartLoading />
        ) : (
          <UsageBarChart
            rows={actionComputeDaily}
            entity="action calls"
            quantityType="actionCompute"
          />
        )}
      </div>
    </Sheet>
  );
}

function FilesUsage({
  team,
  dateRange,
  projectId,
  componentPrefix,
  storage,
  storageEntitlement,
  bandwidth,
  bandwidthEntitlement,
  showEntitlements,
}: {
  team: Team;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
  showEntitlements: boolean;
  storage?: number;
  storageEntitlement?: number | null;
  bandwidth?: number;
  bandwidthEntitlement?: number | null;
}) {
  const filesBandwidth = useUsageTeamStorageThroughputDaily(
    team.id,
    projectId,
    dateRange,
    componentPrefix,
  );

  const fileStorage = useUsageTeamStoragePerDay(
    team.id,
    projectId,
    dateRange,
    componentPrefix,
  );

  const router = useRouter();
  const ref = useRef<HTMLDivElement>(null);

  const [selectedTab, setSelectedTab] = useState(0);

  useEffect(() => {
    const onHashChangeStart = (url: string) => {
      const hash = url.split("#")[1];
      if (hash === "fileStorage" || hash === "fileBandwidth") {
        ref.current?.scrollIntoView({
          behavior: "smooth",
          block: "start",
        });
      }
      if (hash === "fileStorage") {
        setSelectedTab(0);
      }
      if (hash === "fileBandwidth") {
        setSelectedTab(1);
      }
    };

    router.events.on("hashChangeStart", onHashChangeStart);

    return () => {
      router.events.off("hashChangeStart", onHashChangeStart);
    };
  }, [router.events]);

  return (
    <Sheet ref={ref} padding={false} className="pb-4">
      <Tab.Group selectedIndex={selectedTab} onChange={setSelectedTab}>
        <div className="relative mb-4 flex w-full flex-wrap items-center justify-between gap-4 border-b p-4 py-2">
          <h3>Files</h3>

          <Tab.List className="flex gap-2">
            <UsageTab>Storage</UsageTab>
            <UsageTab>Bandwidth</UsageTab>
          </Tab.List>
        </div>
        <Tab.Panels className="px-4">
          <Tab.Panel>
            {showEntitlements && (
              <UsageOverview
                metric={storage}
                entitlement={storageEntitlement ?? 0}
                format={formatBytes}
                showEntitlements={showEntitlements}
              />
            )}
            {fileStorage === undefined ? (
              <ChartLoading />
            ) : fileStorage === null ? (
              <UsageChartUnavailable />
            ) : (
              <UsageStackedBarChart
                rows={fileStorage}
                categories={FILE_STORAGE_CATEGORIES}
                entity="files"
                quantityType="storage"
                showCategoryTotals={false}
              />
            )}
          </Tab.Panel>
          <Tab.Panel>
            {showEntitlements && (
              <UsageOverview
                metric={bandwidth}
                entitlement={bandwidthEntitlement ?? 0}
                format={formatBytes}
                showEntitlements={showEntitlements}
              />
            )}
            {filesBandwidth === undefined ? (
              <ChartLoading />
            ) : (
              <UsageStackedBarChart
                rows={filesBandwidth}
                categories={FILE_BANDWIDTH_CATEGORIES}
                entity="files"
                quantityType="storage"
              />
            )}
          </Tab.Panel>
        </Tab.Panels>
      </Tab.Group>
    </Sheet>
  );
}
function VectorUsage({
  team,
  dateRange,
  projectId,
  componentPrefix,
  storage,
  storageEntitlement,
  bandwidth,
  bandwidthEntitlement,
  showEntitlements,
}: {
  team: Team;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
  storage?: number;
  storageEntitlement?: number | null;
  bandwidth?: number;
  bandwidthEntitlement?: number | null;
  showEntitlements: boolean;
}) {
  const vectorStorage = useUsageTeamVectorStoragePerDay(
    team.id,
    projectId,
    dateRange,
    componentPrefix,
  );
  const vectorBandwidth = useUsageTeamVectorBandwidthPerDay(
    team.id,
    projectId,
    dateRange,
    componentPrefix,
  );

  const router = useRouter();
  const ref = useRef<HTMLDivElement>(null);

  const [selectedTab, setSelectedTab] = useState(0);

  useEffect(() => {
    const onHashChangeStart = (url: string) => {
      const hash = url.split("#")[1];
      if (hash === "vectorStorage" || hash === "vectorBandwidth") {
        ref.current?.scrollIntoView({
          behavior: "smooth",
          block: "start",
        });
      }
      if (hash === "vectorStorage") {
        setSelectedTab(0);
      }
      if (hash === "vectorBandwidth") {
        setSelectedTab(1);
      }
    };

    router.events.on("hashChangeStart", onHashChangeStart);

    return () => {
      router.events.off("hashChangeStart", onHashChangeStart);
    };
  }, [router.events]);

  return (
    <Sheet ref={ref} padding={false} className="pb-4">
      <Tab.Group selectedIndex={selectedTab} onChange={setSelectedTab}>
        <div className="relative mb-4 flex w-full flex-wrap items-center justify-between gap-4 border-b p-4 py-2">
          <h3>Vector Indexes</h3>

          <Tab.List className="flex gap-2">
            <UsageTab>Storage</UsageTab>
            <UsageTab>Bandwidth</UsageTab>
          </Tab.List>
        </div>
        <Tab.Panels className="px-4">
          <Tab.Panel>
            {showEntitlements && (
              <UsageOverview
                metric={storage}
                entitlement={storageEntitlement ?? 0}
                format={formatBytes}
                showEntitlements={showEntitlements}
              />
            )}
            {vectorStorage === undefined ? (
              <ChartLoading />
            ) : vectorStorage === null ? (
              <UsageChartUnavailable />
            ) : (
              <UsageBarChart
                rows={vectorStorage}
                entity="vectors"
                quantityType="storage"
              />
            )}
          </Tab.Panel>
          <Tab.Panel>
            {showEntitlements && (
              <UsageOverview
                metric={bandwidth}
                entitlement={bandwidthEntitlement ?? 0}
                format={formatBytes}
                showEntitlements={showEntitlements}
              />
            )}
            {vectorBandwidth === undefined ? (
              <ChartLoading />
            ) : (
              <UsageStackedBarChart
                rows={vectorBandwidth}
                categories={BANDWIDTH_CATEGORIES}
                entity="vectors"
                quantityType="storage"
              />
            )}
          </Tab.Panel>
        </Tab.Panels>
      </Tab.Group>
    </Sheet>
  );
}

function useHasSubscription(teamId?: number): boolean | undefined {
  const { subscription: orbSub } = useTeamOrbSubscription(teamId);
  return orbSub === undefined ? undefined : orbSub !== null;
}
