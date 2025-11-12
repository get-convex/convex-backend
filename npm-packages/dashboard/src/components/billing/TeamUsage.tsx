import { PlanSummary, UsageOverview } from "components/billing/PlanSummary";
import { Sheet } from "@ui/Sheet";
import { Spinner } from "@ui/Spinner";
import { Button } from "@ui/Button";
import { formatBytes, formatNumberCompact } from "@common/lib/format";
import { sidebarLinkClassNames } from "@common/elements/Sidebar";
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
  DailyMetric,
  DailyMetricByProject,
  DailyPerTagMetrics,
  DailyPerTagMetricsByProject,
} from "hooks/usageMetrics";
import { Team, ProjectDetails } from "generatedApi";
import {
  forwardRef,
  ReactNode,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";
import { useDeployments } from "api/deployments";
import { useTeamEntitlements } from "api/teams";
import { useProjects } from "api/projects";
import { useTeamOrbSubscription } from "api/billing";
import groupBy from "lodash/groupBy";
import sumBy from "lodash/sumBy";
import { Tab } from "@headlessui/react";
import classNames from "classnames";
import { Period } from "elements/UsagePeriodSelector";
import { useRouter } from "next/router";
import { ExternalLinkIcon } from "@radix-ui/react-icons";
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
import { UsageByProjectChart } from "./UsageByProjectChart";
import {
  UsageChartUnavailable,
  UsageDataNotAvailable,
  UsageNoDataError,
  UsageDataError,
} from "./TeamUsageError";
import { TeamUsageToolbar } from "./TeamUsageToolbar";
import { GroupBy, GroupBySelector } from "./GroupBySelector";
import { ProjectLink } from "./ProjectLink";

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

  const { subscription } = useTeamOrbSubscription(team?.id);

  const { data: teamSummary, error: teamSummaryError } = useUsageTeamSummary(
    team?.id,
    shownBillingPeriod
      ? { from: shownBillingPeriod.from, to: shownBillingPeriod.to }
      : null,
    projectId,
    componentPrefix,
  );

  const { data: chefTokenUsage } = useTokenUsage(
    team?.slug,
    shownBillingPeriod,
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
    <div className="[--team-usage-toolbar-height:--spacing(32)] md:[--team-usage-toolbar-height:--spacing(28)] lg:[--team-usage-toolbar-height:--spacing(20)]">
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
              ? team.managedBy
                ? "View Subscription"
                : "View Subscription & Invoices"
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

            <div className="flex flex-col gap-6">
              <PlanSummary
                hasFilter={projectId !== null || !!componentPrefix}
                chefTokenUsage={chefTokenUsage}
                teamSummary={teamSummary}
                entitlements={entitlements}
                hasSubscription={hasSubscription}
                showEntitlements={showEntitlements}
                error={teamSummaryError}
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

              <FunctionBreakdownSection
                team={team}
                dateRange={dateRange}
                projectId={projectId}
                componentPrefix={componentPrefix}
                shownBillingPeriod={shownBillingPeriod}
              />
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
  team: Team;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
  shownBillingPeriod: Period;
}) {
  const projects = useProjects(team.id);

  const { data: metricsByFunction, error: metricsByFunctionError } =
    useUsageTeamMetricsByFunction(
      team.id,
      dateRange,
      projectId,
      componentPrefix,
    );

  const [functionBreakdownTabIndex, setFunctionBreakdownTabIndex] = useState(0);
  const metric = FUNCTION_BREAKDOWN_TABS[functionBreakdownTabIndex];
  const usageByProject = useUsageByProject(metricsByFunction, projects, metric);

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
      header={
        <>
          <h3>Functions breakdown by project</h3>

          <div className="flex flex-wrap items-center gap-4">
            <FunctionBreakdownSelector
              value={functionBreakdownTabIndex}
              onChange={setFunctionBreakdownTabIndex}
            />

            <PaginationControls
              currentPage={currentPage}
              totalPages={totalPages}
              onPageChange={setCurrentPage}
            />
          </div>
        </>
      }
    >
      <div className="px-4">
        {metricsByFunctionError ? (
          <UsageDataError entity="Functions breakdown" />
        ) : !metricsByFunction || !projects ? (
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
  project: ProjectDetails | null;
  rows: AggregatedFunctionMetrics[];
  total: number;
};

function useUsageByProject(
  callsByDeployment: AggregatedFunctionMetrics[] | undefined,
  projects: ProjectDetails[] | undefined,
  metric: FunctionBreakdownMetric,
): UsageInProject[] | undefined {
  return useMemo(() => {
    if (callsByDeployment === undefined || projects === undefined) {
      return undefined;
    }

    const byProject = groupBy(callsByDeployment, (row) => row.projectId);
    return Object.entries(byProject)
      .map(
        ([projectId, rows]): UsageInProject => ({
          key: projectId,
          project: projects.find((p) => p.id === rows[0].projectId) ?? null,
          rows,
          total: sumBy(rows, metric.getTotal),
        }),
      )
      .filter((project) => project.total > 0) // Ignore projects with no data for this metric
      .sort((a, b) => b.total - a.total);
  }, [projects, callsByDeployment, metric]);
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
  team: Team;
}) {
  const maxValue = useMemo(
    () => Math.max(...metricsByDeployment.map(metric.getTotal)),
    [metricsByDeployment, metric],
  );

  if (usageByProject.length === 0) {
    return <UsageNoDataError entity={metric.name} />;
  }

  if (maxValue === 0) {
    return <UsageNoDataError entity={metric.name} />;
  }

  return (
    <div className="scrollbar animate-fadeInFromLoading overflow-y-auto">
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
  const member = useProfile();
  const isLoadingDeployments = project && !deployments;

  return (
    <div className="mb-4">
      <p className="flex align-baseline">
        <ProjectLink project={project} team={team} memberId={member?.id} />
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
  const projects = useProjects(team.id);

  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_database",
    "byType",
  );
  const viewMode = projectId !== null ? "byType" : storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);

  const {
    data: databaseStorageByProject,
    error: databaseStorageByProjectError,
  } = useUsageTeamDatabaseStoragePerDayByProject(
    team.id,
    dateRange,
    componentPrefix,
  );

  const { data: documentsCountByProject, error: documentsCountByProjectError } =
    useUsageTeamDocumentsPerDayByProject(team.id, dateRange, componentPrefix);

  const {
    data: databaseBandwidthByProject,
    error: databaseBandwidthByProjectError,
  } = useUsageTeamDatabaseBandwidthPerDayByProject(
    team.id,
    dateRange,
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

  const databaseBandwidth =
    viewMode === "byType"
      ? aggregateByProjectToByType(databaseBandwidthByProject, projectId)
      : null;

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
    <Tab.Group selectedIndex={selectedTab} onChange={setSelectedTab}>
      <TeamUsageSection
        ref={ref}
        header={
          <>
            <h3>Database</h3>
            <div className="flex flex-wrap items-center gap-2">
              <Tab.List className="flex gap-2">
                <UsageTab>Storage</UsageTab>
                <UsageTab>Bandwidth</UsageTab>
                <UsageTab>Document Count</UsageTab>
              </Tab.List>
              <GroupBySelector
                value={viewMode}
                onChange={setViewMode}
                disabled={projectId !== null}
              />
            </div>
          </>
        }
      >
        <Tab.Panels className="px-4">
          <Tab.Panel>
            {databaseStorageByProjectError ? (
              <UsageDataError entity="Database storage" />
            ) : (
              <>
                {showEntitlements && selectedDate === null && (
                  <UsageOverview
                    metric={storage}
                    entitlement={storageEntitlement ?? 0}
                    format={formatBytes}
                    showEntitlements={showEntitlements}
                  />
                )}
                {viewMode === "byType" ? (
                  databaseStorage === undefined ? (
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
                      selectedDate={selectedDate}
                      setSelectedDate={setSelectedDate}
                    />
                  )
                ) : databaseStorageByProject === undefined ? (
                  <ChartLoading />
                ) : (
                  <UsageByProjectChart
                    rows={databaseStorageByProject}
                    entity="documents"
                    quantityType="storage"
                    projects={projects}
                    team={team}
                    selectedDate={selectedDate}
                    setSelectedDate={setSelectedDate}
                  />
                )}
              </>
            )}
          </Tab.Panel>
          <Tab.Panel>
            {databaseBandwidthByProjectError ? (
              <UsageDataError entity="Database bandwidth" />
            ) : (
              <>
                {showEntitlements && selectedDate === null && (
                  <UsageOverview
                    metric={bandwidth}
                    entitlement={bandwidthEntitlement ?? 0}
                    format={formatBytes}
                    showEntitlements={showEntitlements}
                  />
                )}
                {viewMode === "byType" ? (
                  databaseBandwidth === undefined ? (
                    <ChartLoading />
                  ) : databaseBandwidth === null ? (
                    <UsageChartUnavailable />
                  ) : (
                    <UsageStackedBarChart
                      rows={databaseBandwidth}
                      categories={BANDWIDTH_CATEGORIES}
                      entity="documents"
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
                    entity="documents"
                    quantityType="storage"
                    projects={projects}
                    team={team}
                    selectedDate={selectedDate}
                    setSelectedDate={setSelectedDate}
                  />
                )}
              </>
            )}
          </Tab.Panel>
          <Tab.Panel>
            {documentsCountByProjectError ? (
              <UsageDataError entity="Document count" />
            ) : viewMode === "byType" ? (
              documentsCount === undefined ? (
                <ChartLoading />
              ) : documentsCount === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageBarChart rows={documentsCount} entity="documents" />
              )
            ) : documentsCountByProject === undefined ? (
              <ChartLoading />
            ) : (
              <UsageByProjectChart
                rows={documentsCountByProject}
                entity="documents"
                projects={projects}
                team={team}
                selectedDate={selectedDate}
                setSelectedDate={setSelectedDate}
              />
            )}
          </Tab.Panel>
        </Tab.Panels>
      </TeamUsageSection>
    </Tab.Group>
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
  const projects = useProjects(team.id);

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
    <TeamUsageSection
      ref={ref}
      header={
        <>
          <h3 className="py-2">Daily function calls</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            disabled={projectId !== null}
          />
        </>
      }
    >
      <div className="px-4">
        {callsByTagByProjectError ? (
          <UsageDataError entity="Function calls" />
        ) : (
          <>
            {showEntitlements && selectedDate === null && (
              <UsageOverview
                metric={functionCalls}
                entitlement={functionCallsEntitlement ?? 0}
                format={formatNumberCompact}
                showEntitlements={showEntitlements}
              />
            )}
            {viewMode === "byType" ? (
              callsByTag === undefined ? (
                <ChartLoading />
              ) : callsByTag === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageStackedBarChart
                  rows={callsByTag}
                  entity="calls"
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
                entity="calls"
                projects={projects}
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
  const projects = useProjects(team.id);

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
    <TeamUsageSection
      ref={ref}
      header={
        <>
          <h3 className="py-2">Action Compute</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            disabled={projectId !== null}
          />
        </>
      }
    >
      <div className="px-4">
        {actionComputeDailyByProjectError ? (
          <UsageDataError entity="Action compute" />
        ) : (
          <>
            {showEntitlements && selectedDate === null && (
              <UsageOverview
                metric={actionCompute}
                entitlement={actionComputeEntitlement ?? 0}
                format={formatNumberCompact}
                showEntitlements={showEntitlements}
                suffix="GB-hours"
              />
            )}
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
                entity="action calls"
                quantityType="actionCompute"
                projects={projects}
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
  const projects = useProjects(team.id);

  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_files",
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

  const { data: fileStorageByProject, error: fileStorageByProjectError } =
    useUsageTeamStoragePerDayByProject(team.id, dateRange, componentPrefix);

  const filesBandwidth =
    viewMode === "byType"
      ? aggregateByProjectToByType(filesBandwidthByProject, projectId)
      : null;

  const fileStorage =
    viewMode === "byType"
      ? aggregateByProjectToByType(fileStorageByProject, projectId)
      : null;

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
    <Tab.Group selectedIndex={selectedTab} onChange={setSelectedTab}>
      <TeamUsageSection
        ref={ref}
        header={
          <>
            <h3>Files</h3>
            <div className="flex flex-wrap items-center gap-2">
              <Tab.List className="flex gap-2">
                <UsageTab>Storage</UsageTab>
                <UsageTab>Bandwidth</UsageTab>
              </Tab.List>
              <GroupBySelector
                value={viewMode}
                onChange={setViewMode}
                disabled={projectId !== null}
              />
            </div>
          </>
        }
      >
        <Tab.Panels className="px-4">
          <Tab.Panel>
            {fileStorageByProjectError ? (
              <UsageDataError entity="File storage" />
            ) : (
              <>
                {showEntitlements && selectedDate === null && (
                  <UsageOverview
                    metric={storage}
                    entitlement={storageEntitlement ?? 0}
                    format={formatBytes}
                    showEntitlements={showEntitlements}
                  />
                )}
                {viewMode === "byType" ? (
                  fileStorage === undefined ? (
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
                      selectedDate={selectedDate}
                      setSelectedDate={setSelectedDate}
                    />
                  )
                ) : fileStorageByProject === undefined ? (
                  <ChartLoading />
                ) : (
                  <UsageByProjectChart
                    rows={fileStorageByProject}
                    entity="files"
                    quantityType="storage"
                    projects={projects}
                    team={team}
                    selectedDate={selectedDate}
                    setSelectedDate={setSelectedDate}
                  />
                )}
              </>
            )}
          </Tab.Panel>
          <Tab.Panel>
            {filesBandwidthByProjectError ? (
              <UsageDataError entity="File bandwidth" />
            ) : (
              <>
                {showEntitlements && selectedDate === null && (
                  <UsageOverview
                    metric={bandwidth}
                    entitlement={bandwidthEntitlement ?? 0}
                    format={formatBytes}
                    showEntitlements={showEntitlements}
                  />
                )}
                {viewMode === "byType" ? (
                  filesBandwidth === undefined ? (
                    <ChartLoading />
                  ) : filesBandwidth === null ? (
                    <UsageChartUnavailable />
                  ) : (
                    <UsageStackedBarChart
                      rows={filesBandwidth}
                      categories={FILE_BANDWIDTH_CATEGORIES}
                      entity="files"
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
                    entity="files"
                    quantityType="storage"
                    projects={projects}
                    team={team}
                    selectedDate={selectedDate}
                    setSelectedDate={setSelectedDate}
                  />
                )}
              </>
            )}
          </Tab.Panel>
        </Tab.Panels>
      </TeamUsageSection>
    </Tab.Group>
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
  const projects = useProjects(team.id);

  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_vector",
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

  const {
    data: vectorBandwidthByProject,
    error: vectorBandwidthByProjectError,
  } = useUsageTeamVectorBandwidthPerDayByProject(
    team.id,
    dateRange,
    componentPrefix,
  );

  const vectorStorage =
    viewMode === "byType"
      ? aggregateSimpleByProjectToByType(vectorStorageByProject, projectId)
      : null;

  const vectorBandwidth =
    viewMode === "byType"
      ? aggregateByProjectToByType(vectorBandwidthByProject, projectId)
      : null;

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
    <Tab.Group selectedIndex={selectedTab} onChange={setSelectedTab}>
      <TeamUsageSection
        ref={ref}
        header={
          <>
            <h3>Vector Indexes</h3>
            <div className="flex flex-wrap items-center gap-2">
              <Tab.List className="flex gap-2">
                <UsageTab>Storage</UsageTab>
                <UsageTab>Bandwidth</UsageTab>
              </Tab.List>
              <GroupBySelector
                value={viewMode}
                onChange={setViewMode}
                disabled={projectId !== null}
              />
            </div>
          </>
        }
      >
        <Tab.Panels className="px-4">
          <Tab.Panel>
            {vectorStorageByProjectError ? (
              <UsageDataError entity="Vector storage" />
            ) : (
              <>
                {showEntitlements && selectedDate === null && (
                  <UsageOverview
                    metric={storage}
                    entitlement={storageEntitlement ?? 0}
                    format={formatBytes}
                    showEntitlements={showEntitlements}
                  />
                )}
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
                    entity="vectors"
                    quantityType="storage"
                    projects={projects}
                    team={team}
                    selectedDate={selectedDate}
                    setSelectedDate={setSelectedDate}
                  />
                )}
              </>
            )}
          </Tab.Panel>
          <Tab.Panel>
            {vectorBandwidthByProjectError ? (
              <UsageDataError entity="Vector bandwidth" />
            ) : (
              <>
                {showEntitlements && selectedDate === null && (
                  <UsageOverview
                    metric={bandwidth}
                    entitlement={bandwidthEntitlement ?? 0}
                    format={formatBytes}
                    showEntitlements={showEntitlements}
                  />
                )}
                {viewMode === "byType" ? (
                  vectorBandwidth === undefined ? (
                    <ChartLoading />
                  ) : vectorBandwidth === null ? (
                    <UsageChartUnavailable />
                  ) : (
                    <UsageStackedBarChart
                      rows={vectorBandwidth}
                      categories={BANDWIDTH_CATEGORIES}
                      entity="vectors"
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
                    entity="vectors"
                    quantityType="storage"
                    projects={projects}
                    team={team}
                    selectedDate={selectedDate}
                    setSelectedDate={setSelectedDate}
                  />
                )}
              </>
            )}
          </Tab.Panel>
        </Tab.Panels>
      </TeamUsageSection>
    </Tab.Group>
  );
}

function useHasSubscription(teamId?: number): boolean | undefined {
  const { subscription: orbSub } = useTeamOrbSubscription(teamId);
  return orbSub === undefined ? undefined : orbSub !== null;
}

const TeamUsageSection = forwardRef<
  HTMLDivElement,
  React.PropsWithChildren<{ header: React.ReactNode }>
>(function TeamUsageSection({ header, children }, ref) {
  return (
    <section
      className="scroll-mt-(--section-sticky-top) [--section-sticky-top:calc(var(--team-usage-toolbar-height)_+_--spacing(3))]"
      ref={ref}
    >
      <header
        className={cn(
          "sticky top-(--section-sticky-top) z-10",

          // This pseudo-element makes sure that the contents of the elements aren’t visible above the section header when it is sticky
          "before:absolute before:inset-x-0 before:-top-4 before:h-12 before:bg-background-primary",
        )}
      >
        <div className="relative flex w-full flex-wrap items-center justify-between gap-4 rounded-t-lg border bg-background-secondary p-4 py-2">
          {header}
        </div>
      </header>
      <Sheet padding={false} className="rounded-t-none border-t-0 py-4">
        {children}
      </Sheet>
    </section>
  );
});

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
