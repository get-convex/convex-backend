import { PlanSummary } from "components/billing/PlanSummary";
import { Sheet } from "@ui/Sheet";
import { Spinner } from "@ui/Spinner";
import { Button } from "@ui/Button";
import {
  AggregatedFunctionMetrics,
  useUsageTeamActionComputeDailyByProject,
  useUsageTeamMetricsByFunction,
  useUsageTeamDailyCallsByTagByProject,
  useUsageTeamDatabaseBandwidthPerDayByProject,
  useUsageTeamDocumentsPerDayByProject,
  useUsageTeamDatabaseStoragePerDayByProject,
  useUsageTeamStoragePerDayByProject,
  useUsageTeamStorageThroughputDailyByProject,
  useUsageTeamVectorBandwidthPerDayByProject,
  useUsageTeamVectorStoragePerDayByProject,
  useUsageTeamSummary,
  useTokenUsage,
  useUsageTeamDeploymentCountPerDayByProject,
  useUsageTeamDeploymentCountByType,
  useUsageTeamDatabaseStoragePerDayByTable,
  useUsageTeamDocumentCountPerDayByTable,
  DailyMetric,
  DailyMetricByProject,
  DailyPerTagMetrics,
  DailyPerTagMetricsByProject,
} from "hooks/usageMetrics";
import { TeamResponse } from "generatedApi";
import { useEffect, useMemo, useState } from "react";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";
import { useDeployments } from "api/deployments";
import { useTeamEntitlements } from "api/teams";
import { useProjectById, useProjectBySlug } from "api/projects";
import { useTeamOrbSubscription } from "api/billing";
import groupBy from "lodash/groupBy";
import sumBy from "lodash/sumBy";
import classNames from "classnames";
import { Period } from "elements/UsagePeriodSelector";
import { useRouter } from "next/router";
import { ChevronLeftIcon, ExternalLinkIcon } from "@radix-ui/react-icons";
import { DateRange, useCurrentBillingPeriod } from "api/usage";
import { cn } from "@ui/cn";
import { usePagination } from "hooks/usePagination";
import { PaginationControls } from "elements/PaginationControls";
import { useProfile } from "api/profile";
import { formatQuantity } from "./lib/formatQuantity";
import {
  DATABASE_STORAGE_CATEGORIES,
  BANDWIDTH_CATEGORIES,
  CATEGORY_RENAMES,
  TAG_CATEGORIES,
  FILE_BANDWIDTH_CATEGORIES,
  FILE_STORAGE_CATEGORIES,
} from "./lib/teamUsageCategories";
import {
  FunctionBreakdownMetric,
  FunctionBreakdownMetricActionCompute,
  FunctionBreakdownMetricCalls,
  FunctionBreakdownMetricDatabaseBandwidth,
  FunctionBreakdownMetricVectorBandwidth,
  TeamUsageByFunctionChart,
} from "./TeamUsageByFunctionChart";
import { UsageBarChart, UsageStackedBarChart } from "./UsageBarChart";
import { UsageByProjectChart } from "./UsageByProjectChart";
import { UsageByTableChart } from "./UsageByTableChart";
import {
  UsageChartUnavailable,
  UsageDataNotAvailable,
  UsageNoDataError,
  UsageDataError,
} from "./TeamUsageError";
import { TeamUsageToolbar } from "./TeamUsageToolbar";
import {
  GroupBy,
  DatabaseGroupBy,
  GroupBySelector,
  GROUP_BY_OPTIONS,
  DATABASE_GROUP_BY_OPTIONS,
} from "./GroupBySelector";
import { ProjectLink } from "./ProjectLink";

const FUNCTION_BREAKDOWN_TABS = [
  FunctionBreakdownMetricCalls,
  FunctionBreakdownMetricDatabaseBandwidth,
  FunctionBreakdownMetricActionCompute,
  FunctionBreakdownMetricVectorBandwidth,
];

export type UsageSectionId =
  | "functionCalls"
  | "actionCompute"
  | "databaseStorage"
  | "databaseBandwidth"
  | "databaseDocumentCount"
  | "filesStorage"
  | "filesBandwidth"
  | "vectorsStorage"
  | "vectorsBandwidth"
  | "deployments";

export function TeamUsage({ team }: { team: TeamResponse }) {
  const router = useRouter();
  const { query } = router;
  const project = useProjectBySlug(team.id, query.projectSlug as string);
  const projectId = project?.id ?? null;

  const componentPrefix = (query.componentPrefix ?? null) as string | null;

  const section = (query.section as UsageSectionId) || null;

  const navigateBack = () => {
    const { section: _s, tab: _t, ...restQuery } = query;
    void router.push(
      { pathname: router.pathname, query: restQuery },
      undefined,
      { shallow: true },
    );
  };

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

  const { subscription } = useTeamOrbSubscription(team?.id);

  const { data: teamSummary, error: teamSummaryError } = useUsageTeamSummary(
    team?.id,
    shownBillingPeriod
      ? { from: shownBillingPeriod.from, to: shownBillingPeriod.to }
      : null,
    projectId,
    componentPrefix,
  );

  const { data: deploymentCountData } =
    useUsageTeamDeploymentCountPerDayByProject(
      team?.id,
      dateRange,
      componentPrefix,
    );

  // Get the latest deployment count (highest date)
  const latestDeploymentCount = useMemo(() => {
    if (deploymentCountData === undefined) {
      return undefined;
    }
    if (deploymentCountData.length === 0) {
      return 0;
    }
    // Sort by date descending and get the first item's value, then sum across all projects
    const latestDate = deploymentCountData.reduce(
      (max, item) => (item.ds > max ? item.ds : max),
      deploymentCountData[0].ds,
    );
    return deploymentCountData
      .filter((item) => item.ds === latestDate)
      .reduce((sum, item) => sum + item.value, 0);
  }, [deploymentCountData]);

  const { data: chefTokenUsage } = useTokenUsage(
    team?.slug,
    shownBillingPeriod,
  );

  const entitlements = useTeamEntitlements(team?.id);

  const hasOrbSubscription = useHasSubscription(team?.id);

  // Business plans don't have included usage, so treat them like there's no subscription
  const isBusinessPlan = subscription?.plan.planType === "CONVEX_BUSINESS";

  const hasSubscription =
    (!shownBillingPeriod ||
      shownBillingPeriod.type === "currentBillingPeriod") &&
    (hasOrbSubscription || hasOrbSubscription === undefined) &&
    projectId === null &&
    !isBusinessPlan;

  const showEntitlements =
    (!shownBillingPeriod ||
      shownBillingPeriod.type === "currentBillingPeriod") &&
    projectId === null &&
    !isBusinessPlan;

  return (
    <div className="flex flex-col gap-2 [--team-usage-toolbar-height:--spacing(32)] md:[--team-usage-toolbar-height:--spacing(28)] lg:[--team-usage-toolbar-height:--spacing(20)]">
      <div className="flex justify-between">
        <h2 className="flex items-center gap-2">
          {section && (
            <Button
              variant="neutral"
              size="xs"
              inline
              icon={<ChevronLeftIcon />}
              onClick={navigateBack}
              aria-label="Back to usage overview"
            >
              Back to summary
            </Button>
          )}
          Usage
        </h2>
        {subscription !== undefined && (
          <Button
            href={`/t/${team?.slug}/settings/billing`}
            variant="neutral"
            icon={<ExternalLinkIcon />}
            className="animate-fadeInFromLoading"
            size="xs"
          >
            {subscription
              ? team.managedBy
                ? "View Subscription"
                : "View Subscription & Invoices"
              : "Upgrade Subscription"}
          </Button>
        )}
      </div>

      {currentBillingPeriod !== undefined && shownBillingPeriod !== null && (
        <>
          <TeamUsageToolbar
            {...{
              shownBillingPeriod,
              setSelectedBillingPeriod,
              currentBillingPeriod,
              teamId: team.id,
              projectId,
            }}
          />

          <div className="overflow-x-clip">
            <div
              className={cn(
                "flex gap-6 transition-transform duration-500 motion-reduce:transition-none",
                section ? "-translate-x-[calc(100%+1.5rem)]" : "translate-x-0",
              )}
            >
              {/* Overview pane */}
              <div
                className={cn(
                  "flex w-full shrink-0 flex-col gap-6",
                  section && "pointer-events-none select-none",
                )}
                // @ts-expect-error https://github.com/facebook/react/issues/17157
                inert={section ? "inert" : undefined}
              >
                <PlanSummary
                  hasFilter={projectId !== null || !!componentPrefix}
                  chefTokenUsage={chefTokenUsage}
                  teamSummary={teamSummary}
                  deploymentCount={latestDeploymentCount}
                  entitlements={entitlements}
                  hasSubscription={hasSubscription}
                  showEntitlements={showEntitlements}
                  error={teamSummaryError}
                />

                <FunctionBreakdownSection
                  team={team}
                  dateRange={dateRange}
                  projectId={projectId}
                  componentPrefix={componentPrefix}
                  shownBillingPeriod={shownBillingPeriod}
                />
              </div>

              {/* Detail pane */}
              <div
                className={cn(
                  "flex w-full shrink-0 flex-col gap-6",
                  !section && "pointer-events-none select-none",
                )}
                // @ts-expect-error https://github.com/facebook/react/issues/17157
                inert={!section ? "inert" : undefined}
              >
                {section === "functionCalls" && (
                  <FunctionCallsUsage
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                  />
                )}

                {section === "actionCompute" && (
                  <ActionComputeUsage
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                  />
                )}

                {section === "databaseStorage" && (
                  <DatabaseStorageUsage
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                  />
                )}

                {section === "databaseBandwidth" && (
                  <DatabaseBandwidthUsage
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                  />
                )}

                {section === "databaseDocumentCount" && (
                  <DatabaseDocumentCountUsage
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                  />
                )}

                {section === "filesStorage" && (
                  <FilesStorageUsage
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                  />
                )}

                {section === "filesBandwidth" && (
                  <FilesBandwidthUsage
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                  />
                )}

                {section === "vectorsStorage" && (
                  <VectorStorageUsage
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                  />
                )}

                {section === "vectorsBandwidth" && (
                  <VectorBandwidthUsage
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                  />
                )}

                {section === "deployments" && (
                  <DeploymentCountUsage
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                  />
                )}
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

function FunctionBreakdownSection({
  team,
  dateRange,
  projectId,
  componentPrefix,
  shownBillingPeriod,
}: {
  team: TeamResponse;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
  shownBillingPeriod: Period;
}) {
  const { data: metricsByFunction, error: metricsByFunctionError } =
    useUsageTeamMetricsByFunction(
      team.id,
      dateRange,
      projectId,
      componentPrefix,
    );

  const [functionBreakdownTabIndex, setFunctionBreakdownTabIndex] = useState(0);
  const metric = FUNCTION_BREAKDOWN_TABS[functionBreakdownTabIndex];
  const usageByProject = useUsageByProject(metricsByFunction, metric);

  const {
    visibleItems: visibleProjects,
    totalPages,
    currentPage,
    setCurrentPage,
  } = usePagination({
    items: usageByProject ?? [],
    itemsPerPage: 20,
  });

  // Reset the page number when the filter changes
  useEffect(() => {
    setCurrentPage(1);
  }, [
    team,
    projectId,
    componentPrefix,
    dateRange?.from,
    dateRange?.to,
    shownBillingPeriod.type,
    shownBillingPeriod.from,
    shownBillingPeriod.to,
    functionBreakdownTabIndex,
    setCurrentPage, // stable
  ]);

  const isFunctionBreakdownBandwidthAvailable =
    shownBillingPeriod === null || shownBillingPeriod.from >= "2024-01-01";

  return (
    <TeamUsageSection
      stickyHeader
      header={
        <>
          <h3>Breakdown by function</h3>

          <div className="flex overflow-hidden rounded border">
            {FUNCTION_BREAKDOWN_TABS.map((tab, index) => (
              <Button
                key={tab.name}
                variant="unstyled"
                className={cn(
                  "px-3 py-1 text-sm capitalize",
                  index > 0 && "border-l",
                  functionBreakdownTabIndex === index
                    ? "bg-background-tertiary font-medium"
                    : "text-content-secondary hover:bg-background-tertiary/50",
                )}
                onClick={() => {
                  setFunctionBreakdownTabIndex(index);
                }}
              >
                {tab.name}
              </Button>
            ))}
          </div>

          <PaginationControls
            currentPage={currentPage}
            totalPages={totalPages}
            onPageChange={setCurrentPage}
          />
        </>
      }
    >
      <div className="px-4">
        {metricsByFunctionError ? (
          <UsageDataError entity="Functions breakdown" />
        ) : !metricsByFunction ? (
          <ChartLoading />
        ) : functionBreakdownTabIndex === 0 ||
          isFunctionBreakdownBandwidthAvailable ? (
          <FunctionUsageBreakdown
            team={team}
            usageByProject={visibleProjects}
            metricsByDeployment={metricsByFunction}
            metric={metric}
          />
        ) : (
          <UsageDataNotAvailable
            entity={`Breakdown by ${FUNCTION_BREAKDOWN_TABS[functionBreakdownTabIndex].name}`}
          />
        )}
      </div>
    </TeamUsageSection>
  );
}

type UsageInProject = {
  key: string;
  projectId: number;
  rows: AggregatedFunctionMetrics[];
  total: number;
};

function useUsageByProject(
  callsByDeployment: AggregatedFunctionMetrics[] | undefined,
  metric: FunctionBreakdownMetric,
): UsageInProject[] | undefined {
  return useMemo(() => {
    if (callsByDeployment === undefined) {
      return undefined;
    }

    const byProject = groupBy(callsByDeployment, (row) => row.projectId);
    return Object.entries(byProject)
      .map(
        ([projectId, rows]): UsageInProject => ({
          key: projectId,
          projectId: rows[0].projectId,
          rows,
          total: sumBy(rows, metric.getTotal),
        }),
      )
      .filter((project) => project.total > 0) // Ignore projects with no data for this metric
      .sort((a, b) => b.total - a.total);
  }, [callsByDeployment, metric]);
}

function ChartLoading() {
  return (
    <div className="flex h-56 w-full items-center justify-center">
      <div className="flex items-center justify-center">
        <Spinner className="size-12" />
      </div>
    </div>
  );
}

function FunctionUsageBreakdown({
  usageByProject,
  team,
  metricsByDeployment,
  metric,
}: {
  usageByProject: UsageInProject[];
  metricsByDeployment: AggregatedFunctionMetrics[];
  metric: FunctionBreakdownMetric;
  team: TeamResponse;
}) {
  const maxValue = useMemo(
    () => Math.max(...metricsByDeployment.map(metric.getTotal)),
    [metricsByDeployment, metric],
  );

  if (usageByProject.length === 0) {
    return <UsageNoDataError />;
  }

  if (maxValue === 0) {
    return <UsageNoDataError />;
  }

  return (
    <div className="scrollbar animate-fadeInFromLoading overflow-y-auto">
      {metric.categories !== undefined ? (
        <div className="mb-4 flex items-center gap-6">
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
      {usageByProject.map(({ key, projectId, rows, total }) => (
        <FunctionUsageBreakdownByProject
          key={key}
          projectId={projectId}
          metric={metric}
          rows={rows}
          projectTotal={total}
          maxValue={maxValue}
          team={team}
        />
      ))}
    </div>
  );
}

function FunctionUsageBreakdownByProject({
  projectId,
  metric,
  rows,
  maxValue,
  team,
  projectTotal,
}: {
  projectId: number;
  metric: FunctionBreakdownMetric;
  rows: AggregatedFunctionMetrics[];
  team: TeamResponse;
  maxValue: number;
  projectTotal: number;
}) {
  const { project, isLoading: isLoadingProject } = useProjectById(projectId);
  const { deployments } = useDeployments(projectId);
  const member = useProfile();
  const isLoadingDeployments = !deployments;

  return (
    <div className="mb-4">
      <p className="flex align-baseline">
        <ProjectLink
          project={project ?? null}
          team={team}
          memberId={member?.id}
          isLoading={isLoadingProject}
        />
        <span className="flex-1 px-4 py-2 text-right tabular-nums">
          {formatQuantity(projectTotal, metric.quantityType)}
        </span>
      </p>

      {isLoadingDeployments && <ChartLoading />}
      {!isLoadingDeployments && (
        <TeamUsageByFunctionChart
          project={project ?? null}
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

function DatabaseStorageUsage({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: {
  team: TeamResponse;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
}) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<DatabaseGroupBy>(
    "usageViewMode_databaseStorage",
    "byTable",
  );
  const viewMode = storedViewMode;

  const [activeTab, setActiveTab] = useState<"size" | "count">("size");
  const [selectedDate, setSelectedDate] = useState<number | null>(null);

  const {
    data: databaseStorageByProject,
    error: databaseStorageByProjectError,
  } = useUsageTeamDatabaseStoragePerDayByProject(
    team.id,
    dateRange,
    componentPrefix,
  );

  const { data: databaseStorageByTable, error: databaseStorageByTableError } =
    useUsageTeamDatabaseStoragePerDayByTable(
      team.id,
      dateRange,
      projectId,
      componentPrefix,
    );

  const { data: documentsCountByProject, error: documentsCountByProjectError } =
    useUsageTeamDocumentsPerDayByProject(team.id, dateRange, componentPrefix);

  const { data: documentsCountByTable, error: documentsCountByTableError } =
    useUsageTeamDocumentCountPerDayByTable(
      team.id,
      dateRange,
      projectId,
      componentPrefix,
    );

  const databaseStorage =
    viewMode === "byType"
      ? aggregateByProjectToByType(databaseStorageByProject, projectId)
      : null;

  const documentsCount =
    viewMode === "byType"
      ? aggregateSimpleByProjectToByType(documentsCountByProject, projectId)
      : null;

  const hasError =
    activeTab === "size"
      ? databaseStorageByProjectError || databaseStorageByTableError
      : documentsCountByProjectError || documentsCountByTableError;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Database Storage</h3>
          <div className="flex flex-wrap items-center gap-3">
            <div className="flex overflow-hidden rounded border">
              <Button
                variant="unstyled"
                className={cn(
                  "px-3 py-1 text-sm",
                  activeTab === "size"
                    ? "bg-background-tertiary font-medium"
                    : "text-content-secondary hover:bg-background-tertiary/50",
                )}
                onClick={() => {
                  setActiveTab("size");
                  setSelectedDate(null);
                }}
              >
                Document Size
              </Button>
              <Button
                variant="unstyled"
                className={cn(
                  "border-l px-3 py-1 text-sm",
                  activeTab === "count"
                    ? "bg-background-tertiary font-medium"
                    : "text-content-secondary hover:bg-background-tertiary/50",
                )}
                onClick={() => {
                  setActiveTab("count");
                  setSelectedDate(null);
                }}
              >
                Document Count
              </Button>
            </div>
            <GroupBySelector
              value={viewMode}
              onChange={setViewMode}
              disabled={false}
              options={DATABASE_GROUP_BY_OPTIONS}
            />
          </div>
        </>
      }
    >
      <div className="px-4">
        {hasError ? (
          <UsageDataError entity="Database storage" />
        ) : activeTab === "size" ? (
          <>
            {viewMode === "byType" ? (
              databaseStorage === undefined ? (
                <ChartLoading />
              ) : databaseStorage === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageStackedBarChart
                  rows={databaseStorage}
                  categories={DATABASE_STORAGE_CATEGORIES}
                  quantityType="storage"
                  showCategoryTotals={false}
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : viewMode === "byTable" ? (
              databaseStorageByTable === undefined ? (
                <ChartLoading />
              ) : databaseStorageByTable === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageByTableChart
                  rows={databaseStorageByTable}
                  quantityType="storage"
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : databaseStorageByProject === undefined ? (
              <ChartLoading />
            ) : (
              <UsageByProjectChart
                rows={databaseStorageByProject}
                quantityType="storage"
                team={team}
                selectedDate={selectedDate}
                setSelectedDate={setSelectedDate}
              />
            )}
          </>
        ) : (
          <>
            {viewMode === "byType" ? (
              documentsCount === undefined ? (
                <ChartLoading />
              ) : documentsCount === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageBarChart rows={documentsCount} entity="documents" />
              )
            ) : viewMode === "byTable" ? (
              documentsCountByTable === undefined ? (
                <ChartLoading />
              ) : documentsCountByTable === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageByTableChart
                  rows={documentsCountByTable}
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : documentsCountByProject === undefined ? (
              <ChartLoading />
            ) : (
              <UsageByProjectChart
                rows={documentsCountByProject}
                team={team}
                selectedDate={selectedDate}
                setSelectedDate={setSelectedDate}
              />
            )}
          </>
        )}
      </div>
    </TeamUsageSection>
  );
}

function DatabaseBandwidthUsage({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: {
  team: TeamResponse;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
}) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_databaseBandwidth",
    "byType",
  );
  const viewMode = projectId !== null ? "byType" : storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);

  const {
    data: databaseBandwidthByProject,
    error: databaseBandwidthByProjectError,
  } = useUsageTeamDatabaseBandwidthPerDayByProject(
    team.id,
    dateRange,
    componentPrefix,
  );

  const databaseBandwidth =
    viewMode === "byType"
      ? aggregateByProjectToByType(databaseBandwidthByProject, projectId)
      : null;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Database Bandwidth</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            disabled={projectId !== null}
            options={GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {databaseBandwidthByProjectError ? (
          <UsageDataError entity="Database bandwidth" />
        ) : (
          <>
            {viewMode === "byType" ? (
              databaseBandwidth === undefined ? (
                <ChartLoading />
              ) : databaseBandwidth === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageStackedBarChart
                  rows={databaseBandwidth}
                  categories={BANDWIDTH_CATEGORIES}
                  quantityType="storage"
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : databaseBandwidthByProject === undefined ? (
              <ChartLoading />
            ) : (
              <UsageByProjectChart
                rows={databaseBandwidthByProject}
                quantityType="storage"
                team={team}
                selectedDate={selectedDate}
                setSelectedDate={setSelectedDate}
              />
            )}
          </>
        )}
      </div>
    </TeamUsageSection>
  );
}

function DatabaseDocumentCountUsage({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: {
  team: TeamResponse;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
}) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<DatabaseGroupBy>(
    "usageViewMode_databaseDocumentCount",
    "byTable",
  );
  const viewMode = storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);

  const { data: documentsCountByProject, error: documentsCountByProjectError } =
    useUsageTeamDocumentsPerDayByProject(team.id, dateRange, componentPrefix);

  const { data: documentsCountByTable, error: documentsCountByTableError } =
    useUsageTeamDocumentCountPerDayByTable(
      team.id,
      dateRange,
      projectId,
      componentPrefix,
    );

  const documentsCount =
    viewMode === "byType"
      ? aggregateSimpleByProjectToByType(documentsCountByProject, projectId)
      : null;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Database Document Count</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            disabled={false}
            options={DATABASE_GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {documentsCountByProjectError || documentsCountByTableError ? (
          <UsageDataError entity="Document count" />
        ) : viewMode === "byType" ? (
          documentsCount === undefined ? (
            <ChartLoading />
          ) : documentsCount === null ? (
            <UsageChartUnavailable />
          ) : (
            <UsageBarChart rows={documentsCount} entity="documents" />
          )
        ) : viewMode === "byTable" ? (
          documentsCountByTable === undefined ? (
            <ChartLoading />
          ) : documentsCountByTable === null ? (
            <UsageChartUnavailable />
          ) : (
            <UsageByTableChart
              rows={documentsCountByTable}
              selectedDate={selectedDate}
              setSelectedDate={setSelectedDate}
            />
          )
        ) : documentsCountByProject === undefined ? (
          <ChartLoading />
        ) : (
          <UsageByProjectChart
            rows={documentsCountByProject}
            team={team}
            selectedDate={selectedDate}
            setSelectedDate={setSelectedDate}
          />
        )}
      </div>
    </TeamUsageSection>
  );
}

function FunctionCallsUsage({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: {
  team: TeamResponse;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
}) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_functionCalls",
    "byType",
  );
  const viewMode = projectId !== null ? "byType" : storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);

  const { data: callsByTagByProject, error: callsByTagByProjectError } =
    useUsageTeamDailyCallsByTagByProject(team.id, dateRange, componentPrefix);

  const callsByTag =
    viewMode === "byType"
      ? aggregateByProjectToByType(callsByTagByProject, projectId)
      : null;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Daily function calls</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            disabled={projectId !== null}
            options={GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {callsByTagByProjectError ? (
          <UsageDataError entity="Function calls" />
        ) : (
          <>
            {viewMode === "byType" ? (
              callsByTag === undefined ? (
                <ChartLoading />
              ) : callsByTag === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageStackedBarChart
                  rows={callsByTag}
                  categories={TAG_CATEGORIES}
                  categoryRenames={CATEGORY_RENAMES}
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : callsByTagByProject === undefined ? (
              <ChartLoading />
            ) : (
              <UsageByProjectChart
                rows={callsByTagByProject}
                team={team}
                selectedDate={selectedDate}
                setSelectedDate={setSelectedDate}
              />
            )}
          </>
        )}
      </div>
    </TeamUsageSection>
  );
}

function ActionComputeUsage({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: {
  team: TeamResponse;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
}) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_actionCompute",
    "byType",
  );
  const viewMode = projectId !== null ? "byType" : storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);

  const {
    data: actionComputeDailyByProject,
    error: actionComputeDailyByProjectError,
  } = useUsageTeamActionComputeDailyByProject(
    team.id,
    dateRange,
    componentPrefix,
  );

  const actionComputeDaily =
    viewMode === "byType"
      ? aggregateSimpleByProjectToByType(actionComputeDailyByProject, projectId)
      : null;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Action Compute</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            disabled={projectId !== null}
            options={GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {actionComputeDailyByProjectError ? (
          <UsageDataError entity="Action compute" />
        ) : (
          <>
            {viewMode === "byType" ? (
              actionComputeDaily === undefined ? (
                <ChartLoading />
              ) : actionComputeDaily === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageBarChart
                  rows={actionComputeDaily}
                  entity="action calls"
                  quantityType="actionCompute"
                />
              )
            ) : actionComputeDailyByProject === undefined ? (
              <ChartLoading />
            ) : (
              <UsageByProjectChart
                rows={actionComputeDailyByProject}
                quantityType="actionCompute"
                team={team}
                selectedDate={selectedDate}
                setSelectedDate={setSelectedDate}
              />
            )}
          </>
        )}
      </div>
    </TeamUsageSection>
  );
}

function FilesStorageUsage({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: {
  team: TeamResponse;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
}) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_filesStorage",
    "byType",
  );
  const viewMode = projectId !== null ? "byType" : storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);

  const { data: fileStorageByProject, error: fileStorageByProjectError } =
    useUsageTeamStoragePerDayByProject(team.id, dateRange, componentPrefix);

  const fileStorage =
    viewMode === "byType"
      ? aggregateByProjectToByType(fileStorageByProject, projectId)
      : null;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">File Storage</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            disabled={projectId !== null}
            options={GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {fileStorageByProjectError ? (
          <UsageDataError entity="File storage" />
        ) : (
          <>
            {viewMode === "byType" ? (
              fileStorage === undefined ? (
                <ChartLoading />
              ) : fileStorage === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageStackedBarChart
                  rows={fileStorage}
                  categories={FILE_STORAGE_CATEGORIES}
                  quantityType="storage"
                  showCategoryTotals={false}
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : fileStorageByProject === undefined ? (
              <ChartLoading />
            ) : (
              <UsageByProjectChart
                rows={fileStorageByProject}
                quantityType="storage"
                team={team}
                selectedDate={selectedDate}
                setSelectedDate={setSelectedDate}
              />
            )}
          </>
        )}
      </div>
    </TeamUsageSection>
  );
}

function FilesBandwidthUsage({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: {
  team: TeamResponse;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
}) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_filesBandwidth",
    "byType",
  );
  const viewMode = projectId !== null ? "byType" : storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);

  const { data: filesBandwidthByProject, error: filesBandwidthByProjectError } =
    useUsageTeamStorageThroughputDailyByProject(
      team.id,
      dateRange,
      componentPrefix,
    );

  const filesBandwidth =
    viewMode === "byType"
      ? aggregateByProjectToByType(filesBandwidthByProject, projectId)
      : null;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">File Bandwidth</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            disabled={projectId !== null}
            options={GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {filesBandwidthByProjectError ? (
          <UsageDataError entity="File bandwidth" />
        ) : (
          <>
            {viewMode === "byType" ? (
              filesBandwidth === undefined ? (
                <ChartLoading />
              ) : filesBandwidth === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageStackedBarChart
                  rows={filesBandwidth}
                  categories={FILE_BANDWIDTH_CATEGORIES}
                  quantityType="storage"
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : filesBandwidthByProject === undefined ? (
              <ChartLoading />
            ) : (
              <UsageByProjectChart
                rows={filesBandwidthByProject}
                quantityType="storage"
                team={team}
                selectedDate={selectedDate}
                setSelectedDate={setSelectedDate}
              />
            )}
          </>
        )}
      </div>
    </TeamUsageSection>
  );
}
function DeploymentCountUsage({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: {
  team: TeamResponse;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
}) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_deploymentCount",
    "byType",
  );
  const viewMode = projectId !== null ? "byType" : storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);

  const {
    data: deploymentCountDailyByProject,
    error: deploymentCountDailyByProjectError,
  } = useUsageTeamDeploymentCountPerDayByProject(
    team.id,
    dateRange,
    componentPrefix,
  );

  const { data: deploymentCountByType, error: deploymentCountByTypeError } =
    useUsageTeamDeploymentCountByType(
      team.id,
      dateRange,
      projectId,
      componentPrefix,
    );

  const deploymentTypeCategories = {
    prod: {
      name: "Production",
      color: "fill-chart-line-1",
    },
    dev: {
      name: "Development",
      color: "fill-chart-line-2",
    },
    preview: {
      name: "Preview",
      color: "fill-chart-line-3",
    },
    deleted: {
      name: "Deleted Deployment",
      color: "fill-chart-line-4",
    },
  };

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Deployments</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            disabled={projectId !== null}
            options={GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {viewMode === "byType" ? (
          // Show deployment type breakdown (prod/dev/preview/deleted)
          deploymentCountByTypeError ? (
            <UsageDataError entity="Deployments" />
          ) : deploymentCountByType === undefined ? (
            <ChartLoading />
          ) : (
            <UsageStackedBarChart
              rows={deploymentCountByType}
              categories={deploymentTypeCategories}
              showCategoryTotals={false}
              selectedDate={selectedDate}
              setSelectedDate={setSelectedDate}
            />
          )
        ) : // Show deployment count by project
        deploymentCountDailyByProjectError ? (
          <UsageDataError entity="Deployments" />
        ) : deploymentCountDailyByProject === undefined ? (
          <ChartLoading />
        ) : (
          <UsageByProjectChart
            rows={deploymentCountDailyByProject}
            team={team}
            selectedDate={selectedDate}
            setSelectedDate={setSelectedDate}
          />
        )}
      </div>
    </TeamUsageSection>
  );
}

function VectorStorageUsage({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: {
  team: TeamResponse;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
}) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_vectorsStorage",
    "byType",
  );
  const viewMode = projectId !== null ? "byType" : storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);

  const { data: vectorStorageByProject, error: vectorStorageByProjectError } =
    useUsageTeamVectorStoragePerDayByProject(
      team.id,
      dateRange,
      componentPrefix,
    );

  const vectorStorage =
    viewMode === "byType"
      ? aggregateSimpleByProjectToByType(vectorStorageByProject, projectId)
      : null;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Vector Index Storage</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            disabled={projectId !== null}
            options={GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {vectorStorageByProjectError ? (
          <UsageDataError entity="Vector storage" />
        ) : (
          <>
            {viewMode === "byType" ? (
              vectorStorage === undefined ? (
                <ChartLoading />
              ) : vectorStorage === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageBarChart
                  rows={vectorStorage}
                  entity="vectors"
                  quantityType="storage"
                />
              )
            ) : vectorStorageByProject === undefined ? (
              <ChartLoading />
            ) : (
              <UsageByProjectChart
                rows={vectorStorageByProject}
                quantityType="storage"
                team={team}
                selectedDate={selectedDate}
                setSelectedDate={setSelectedDate}
              />
            )}
          </>
        )}
      </div>
    </TeamUsageSection>
  );
}

function VectorBandwidthUsage({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: {
  team: TeamResponse;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
}) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_vectorsBandwidth",
    "byType",
  );
  const viewMode = projectId !== null ? "byType" : storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);

  const {
    data: vectorBandwidthByProject,
    error: vectorBandwidthByProjectError,
  } = useUsageTeamVectorBandwidthPerDayByProject(
    team.id,
    dateRange,
    componentPrefix,
  );

  const vectorBandwidth =
    viewMode === "byType"
      ? aggregateByProjectToByType(vectorBandwidthByProject, projectId)
      : null;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Vector Index Bandwidth</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            disabled={projectId !== null}
            options={GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {vectorBandwidthByProjectError ? (
          <UsageDataError entity="Vector bandwidth" />
        ) : (
          <>
            {viewMode === "byType" ? (
              vectorBandwidth === undefined ? (
                <ChartLoading />
              ) : vectorBandwidth === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageStackedBarChart
                  rows={vectorBandwidth}
                  categories={BANDWIDTH_CATEGORIES}
                  quantityType="storage"
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : vectorBandwidthByProject === undefined ? (
              <ChartLoading />
            ) : (
              <UsageByProjectChart
                rows={vectorBandwidthByProject}
                quantityType="storage"
                team={team}
                selectedDate={selectedDate}
                setSelectedDate={setSelectedDate}
              />
            )}
          </>
        )}
      </div>
    </TeamUsageSection>
  );
}

function useHasSubscription(teamId?: number): boolean | undefined {
  const { subscription: orbSub } = useTeamOrbSubscription(teamId);
  return orbSub === undefined ? undefined : orbSub !== null;
}

function TeamUsageSection({
  header,
  children,
  stickyHeader,
}: React.PropsWithChildren<{
  header: React.ReactNode;
  stickyHeader?: boolean;
}>) {
  return (
    <section>
      <header
        className={
          stickyHeader
            ? "sticky top-(--team-usage-toolbar-height) z-10 bg-background-primary pt-2"
            : undefined
        }
      >
        <div className="flex w-full flex-wrap items-center justify-between gap-4 rounded-t-lg border bg-background-secondary p-4 py-2">
          {header}
        </div>
      </header>
      <Sheet padding={false} className="rounded-t-none border-t-0 py-4">
        {children}
      </Sheet>
    </section>
  );
}

// Aggregate by-project data to by-type view by summing across all projects
function aggregateByProjectToByType(
  rows: DailyPerTagMetricsByProject[] | undefined | null,
  projectId: number | null,
): DailyPerTagMetrics[] | undefined | null {
  if (rows === undefined) return undefined;
  if (rows === null) return null;

  // If a project filter is active, filter to that project
  const filteredRows =
    projectId === null
      ? rows
      : rows.filter((row) => row.projectId === projectId);

  const grouped = groupBy(filteredRows, (row) => row.ds);
  return Object.entries(grouped).map(([ds, dayRows]) => {
    // For each day, aggregate metrics across all projects
    const metricsMap = new Map<string, number>();
    for (const row of dayRows) {
      for (const metric of row.metrics) {
        metricsMap.set(
          metric.tag,
          (metricsMap.get(metric.tag) || 0) + metric.value,
        );
      }
    }
    return {
      ds,
      metrics: Array.from(metricsMap.entries()).map(([tag, value]) => ({
        tag,
        value,
      })),
    };
  });
}

// Aggregate simple by-project data to by-type view
function aggregateSimpleByProjectToByType(
  rows: DailyMetricByProject[] | undefined | null,
  projectId: number | null,
): DailyMetric[] | undefined | null {
  if (rows === undefined) return undefined;
  if (rows === null) return null;
  // If a project filter is active, filter to that project
  const filteredRows =
    projectId === null
      ? rows
      : rows.filter((row) => row.projectId === projectId);

  const grouped = groupBy(filteredRows, (row) => row.ds);
  return Object.entries(grouped).map(([ds, dayRows]) => ({
    ds,
    value: sumBy(dayRows, (row) => row.value),
  }));
}
