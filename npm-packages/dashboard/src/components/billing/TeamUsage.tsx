import {
  PlanSummary,
  BusinessPlanSummary,
} from "components/billing/PlanSummary";
import { Sheet } from "@ui/Sheet";
import { Spinner } from "@ui/Spinner";
import { Button } from "@ui/Button";
import { Callout } from "@ui/Callout";
import { SegmentedControl } from "@ui/SegmentedControl";
import {
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
import { Period } from "elements/UsagePeriodSelector";
import { useRouter } from "next/router";
import { Link } from "@ui/Link";
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
  DATA_EGRESS_CATEGORIES,
  DATA_EGRESS_CATEGORY_RENAMES,
  COMPUTE_CATEGORIES_SELF_SERVE,
  SEARCH_STORAGE_CATEGORIES,
  SEARCH_QUERIES_CATEGORIES,
  DATABASE_IO_CATEGORIES,
  COMPUTE_CATEGORIES,
  DEPLOYMENT_CLASS_CATEGORIES,
} from "./lib/teamUsageCategories";
import {
  FunctionBreakdownMetric,
  FunctionBreakdownMetricActionCompute,
  FunctionBreakdownMetricCalls,
  FunctionBreakdownMetricDatabaseBandwidth,
  FunctionBreakdownMetricVectorBandwidth,
  FunctionMetricsRow,
  FunctionBreakdownMetricCallsV2,
  FunctionBreakdownMetricDatabaseIOV2,
  FunctionBreakdownMetricComputeV2,
  FunctionBreakdownMetricSearchV2,
  FunctionBreakdownMetricDataEgressV2,
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
  BusinessGroupBy,
  BusinessDatabaseGroupBy,
  GroupBySelector,
  GROUP_BY_OPTIONS,
  DATABASE_GROUP_BY_OPTIONS,
  BUSINESS_GROUP_BY_OPTIONS,
  BUSINESS_DATABASE_GROUP_BY_OPTIONS,
} from "./GroupBySelector";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { ProjectLink } from "./ProjectLink";
import {
  useUsageTeamSummaryV2,
  useUsageTeamMetricsByFunctionV2,
  useDatabaseStoragePerDayByProjectAndClassV2,
  useDatabaseStoragePerDayByTableV2,
  useDocumentCountPerDayByTableV2,
  useDatabaseIOPerDayByProjectAndClassV2,
  useFunctionCallsPerDayByProjectAndClassV2,
  useComputePerDayByProjectV2,
  useFileStoragePerDayByProjectV2,
  useSearchStoragePerDayByProjectV2,
  useDataEgressPerDayByProjectV2,
  useSearchQueriesPerDayByProjectV2,
  useDeploymentsByClassAndRegionV2,
  useComputePerDayByProjectSelfServeV2,
  DailyPerTagMetricsByProjectAndClass,
} from "hooks/usageMetricsV2";

const FUNCTION_BREAKDOWN_TABS = [
  FunctionBreakdownMetricCalls,
  FunctionBreakdownMetricDatabaseBandwidth,
  FunctionBreakdownMetricActionCompute,
  FunctionBreakdownMetricVectorBandwidth,
];

const FUNCTION_BREAKDOWN_TABS_V2 = [
  FunctionBreakdownMetricCallsV2,
  FunctionBreakdownMetricDatabaseIOV2,
  FunctionBreakdownMetricComputeV2,
  FunctionBreakdownMetricSearchV2,
  FunctionBreakdownMetricDataEgressV2,
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
  | "deployments"
  // Business plan sections
  | "compute"
  | "databaseIO"
  | "searchStorage"
  | "searchQueries"
  | "dataEgress";

export function TeamUsage({ team }: { team: TeamResponse }) {
  const router = useRouter();
  const { query } = router;
  const project = useProjectBySlug(team.id, query.projectSlug as string);
  const projectId = project?.id ?? null;

  const componentPrefix = (query.componentPrefix ?? null) as string | null;

  const section = (query.section as UsageSectionId) || null;

  const SECTION_TITLES: Record<UsageSectionId, string> = {
    functionCalls: "Function Calls",
    actionCompute: "Action Compute",
    databaseStorage: "Database Storage",
    databaseBandwidth: "Database Bandwidth",
    databaseDocumentCount: "Document Count",
    filesStorage: "File Storage",
    filesBandwidth: "File Bandwidth",
    vectorsStorage: "Vector Storage",
    vectorsBandwidth: "Vector Bandwidth",
    deployments: "Deployments",
    compute: "Compute",
    databaseIO: "Database I/O",
    searchStorage: "Search Storage",
    searchQueries: "Search Queries",
    dataEgress: "Data Egress",
  };

  const summaryHref = (() => {
    const { section: _s, tab: _t, ...restQuery } = query;
    return { pathname: router.pathname, query: restQuery };
  })();

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

  const { usageDashboardV2 } = useLaunchDarkly();

  const isBusinessPlanType = subscription?.plan.planType === "CONVEX_BUSINESS";
  const hasNewBilling = subscription?.hasNewBilling ?? false;
  const [previewNewBilling, setPreviewNewBilling] = useState(false);
  // When the V2 flag is on, use V2 for teams on new billing or previewing it
  const useV2 = usageDashboardV2 && (hasNewBilling || previewNewBilling);

  const billingPeriodRange = shownBillingPeriod
    ? { from: shownBillingPeriod.from, to: shownBillingPeriod.to }
    : null;

  const { data: teamSummary, error: teamSummaryError } = useUsageTeamSummary(
    team?.id,
    billingPeriodRange,
    projectId,
    componentPrefix,
  );

  const { data: summaryV2, error: summaryV2Error } = useUsageTeamSummaryV2(
    team?.id,
    billingPeriodRange,
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

  const hasSubscription =
    (!shownBillingPeriod ||
      shownBillingPeriod.type === "currentBillingPeriod") &&
    (hasOrbSubscription || hasOrbSubscription === undefined) &&
    projectId === null &&
    !isBusinessPlanType;

  const showEntitlements =
    (!shownBillingPeriod ||
      shownBillingPeriod.type === "currentBillingPeriod") &&
    projectId === null &&
    !isBusinessPlanType;

  return (
    <div className="flex min-w-[40rem] flex-col gap-2 [--team-usage-toolbar-height:--spacing(32)] md:[--team-usage-toolbar-height:--spacing(28)] lg:[--team-usage-toolbar-height:--spacing(20)]">
      <div className="flex justify-between">
        <h2 className="flex items-center gap-2">
          {section ? (
            <>
              <Link href={summaryHref}>Usage</Link>
              <span className="animate-fadeInFromLoading">/</span>
              <span className="animate-fadeInFromLoading">
                {SECTION_TITLES[section]}
              </span>
            </>
          ) : (
            "Usage"
          )}
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
              ? team.managedBy === "vercel"
                ? "View Subscription"
                : "View Subscription & Invoices"
              : "Upgrade Subscription"}
          </Button>
        )}
      </div>

      {usageDashboardV2 &&
        !hasNewBilling &&
        !isBusinessPlanType &&
        subscription && (
          <Callout variant="hint">
            <div className="flex w-full items-center justify-between gap-4">
              <span>
                Your Convex subscription pricing is changing{" "}
                {subscription.newBillingStartDate
                  ? `on ${new Date(subscription.newBillingStartDate + "T00:00:00").toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" })}`
                  : "in May 2026"}
                .
                <div className="mt-1 flex gap-3">
                  <Link
                    href="https://convex.dev/pricing"
                    target="_blank"
                    className="text-util-accent hover:underline dark:text-white"
                  >
                    Go to pricing page
                  </Link>
                  <Link
                    href="https://news.convex.dev/enterprise-launch/"
                    target="_blank"
                    className="text-util-accent hover:underline dark:text-white"
                  >
                    View blog post
                  </Link>
                </div>
              </span>
              {/* eslint-disable-next-line jsx-a11y/label-has-associated-control -- custom toggle switch */}
              <label className="flex shrink-0 cursor-pointer items-center gap-2 text-sm">
                <span>Preview new usage metrics</span>
                {/* eslint-disable-next-line react/forbid-elements -- custom toggle switch, not a standard button */}
                <button
                  type="button"
                  role="switch"
                  aria-checked={previewNewBilling}
                  aria-label="Preview new usage metrics"
                  onClick={() => {
                    setPreviewNewBilling((prev) => !prev);
                    if (section) {
                      void router.push(summaryHref, undefined, {
                        shallow: true,
                      });
                    }
                  }}
                  className={cn(
                    "relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors",
                    "focus-visible:outline-2 focus-visible:outline-border-selected",
                    previewNewBilling
                      ? "bg-util-accent"
                      : "bg-neutral-4 dark:bg-neutral-7",
                  )}
                >
                  <span
                    className={cn(
                      "inline-block h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform",
                      previewNewBilling
                        ? "translate-x-[18px]"
                        : "translate-x-[3px]",
                    )}
                  />
                </button>
              </label>
            </div>
          </Callout>
        )}

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
                  section &&
                    "pointer-events-none h-0 overflow-hidden select-none",
                )}
                // @ts-expect-error https://github.com/facebook/react/issues/17157
                inert={section ? "inert" : undefined}
              >
                {useV2 ? (
                  <>
                    <BusinessPlanSummary
                      hasFilter={projectId !== null || !!componentPrefix}
                      summaryV2={summaryV2}
                      deploymentCount={latestDeploymentCount}
                      chefTokenUsage={chefTokenUsage}
                      error={summaryV2Error}
                      isBusinessPlan={isBusinessPlanType}
                      entitlements={entitlements}
                      hasSubscription={hasSubscription}
                      showEntitlements={showEntitlements}
                    />
                    <FunctionBreakdownSectionV2
                      team={team}
                      dateRange={dateRange}
                      projectId={projectId}
                      componentPrefix={componentPrefix}
                      shownBillingPeriod={shownBillingPeriod}
                    />
                  </>
                ) : (
                  <>
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
                  </>
                )}
              </div>

              {/* Detail pane */}
              <div
                className={cn(
                  "flex w-full shrink-0 flex-col gap-6",
                  !section &&
                    "pointer-events-none h-0 overflow-hidden select-none",
                )}
                // @ts-expect-error https://github.com/facebook/react/issues/17157
                inert={!section ? "inert" : undefined}
              >
                {section === "functionCalls" &&
                  (useV2 ? (
                    <FunctionCallsUsageV2
                      team={team}
                      dateRange={dateRange}
                      projectId={projectId}
                      componentPrefix={componentPrefix}
                    />
                  ) : (
                    <FunctionCallsUsage
                      team={team}
                      dateRange={dateRange}
                      projectId={projectId}
                      componentPrefix={componentPrefix}
                    />
                  ))}

                {/* V1-only: self-serve action compute section */}
                {section === "actionCompute" && !useV2 && (
                  <ActionComputeUsage
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                  />
                )}
                {/* V2: self-serve action compute uses the same section ID */}
                {section === "actionCompute" && useV2 && (
                  <ComputeUsageV2
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                    isBusinessPlan={false}
                  />
                )}

                {section === "databaseStorage" &&
                  (useV2 ? (
                    <DatabaseStorageUsageV2
                      team={team}
                      dateRange={dateRange}
                      projectId={projectId}
                      componentPrefix={componentPrefix}
                    />
                  ) : (
                    <DatabaseStorageUsage
                      team={team}
                      dateRange={dateRange}
                      projectId={projectId}
                      componentPrefix={componentPrefix}
                    />
                  ))}

                {/* V1-only sections */}
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

                {section === "filesStorage" &&
                  (useV2 ? (
                    <FileStorageUsageV2
                      team={team}
                      dateRange={dateRange}
                      projectId={projectId}
                      componentPrefix={componentPrefix}
                    />
                  ) : (
                    <FilesStorageUsage
                      team={team}
                      dateRange={dateRange}
                      projectId={projectId}
                      componentPrefix={componentPrefix}
                    />
                  ))}

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

                {section === "deployments" &&
                  (useV2 ? (
                    <DeploymentCountUsageV2
                      team={team}
                      dateRange={dateRange}
                      projectId={projectId}
                      componentPrefix={componentPrefix}
                    />
                  ) : (
                    <DeploymentCountUsage
                      team={team}
                      dateRange={dateRange}
                      projectId={projectId}
                      componentPrefix={componentPrefix}
                    />
                  ))}

                {/* V2 detail sections */}
                {section === "compute" && (
                  <ComputeUsageV2
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                    isBusinessPlan={true}
                  />
                )}

                {section === "databaseIO" && (
                  <DatabaseIOUsageV2
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                  />
                )}

                {section === "searchStorage" && (
                  <SearchStorageUsageV2
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                  />
                )}

                {section === "searchQueries" && (
                  <SearchQueriesUsageV2
                    team={team}
                    dateRange={dateRange}
                    projectId={projectId}
                    componentPrefix={componentPrefix}
                  />
                )}

                {section === "dataEgress" && (
                  <DataEgressUsageV2
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

  const [functionBreakdownTab, setFunctionBreakdownTab] = useState(
    FUNCTION_BREAKDOWN_TABS[0].name,
  );
  const metric =
    FUNCTION_BREAKDOWN_TABS.find((t) => t.name === functionBreakdownTab) ??
    FUNCTION_BREAKDOWN_TABS[0];
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
    functionBreakdownTab,
    setCurrentPage, // stable
  ]);

  const isFunctionBreakdownBandwidthAvailable =
    shownBillingPeriod === null || shownBillingPeriod.from >= "2024-01-01";

  const functionBreakdownOptions = FUNCTION_BREAKDOWN_TABS.map((tab) => ({
    label: tab.name.replace(/\b\w/g, (c) => c.toUpperCase()),
    value: tab.name,
  }));

  return (
    <TeamUsageSection
      stickyHeader
      header={
        <div className="flex w-full flex-col gap-2">
          <div className="flex items-center justify-between gap-4">
            <h3>Breakdown by function</h3>

            <SegmentedControl
              options={functionBreakdownOptions}
              value={functionBreakdownTab}
              onChange={setFunctionBreakdownTab}
            />
          </div>

          {totalPages > 1 && (
            <div className="flex justify-end">
              <PaginationControls
                currentPage={currentPage}
                totalPages={totalPages}
                onPageChange={setCurrentPage}
              />
            </div>
          )}
        </div>
      }
    >
      <div className="px-4">
        {metricsByFunctionError ? (
          <UsageDataError entity="Functions breakdown" />
        ) : !metricsByFunction ? (
          <ChartLoading />
        ) : metric === FUNCTION_BREAKDOWN_TABS[0] ||
          isFunctionBreakdownBandwidthAvailable ? (
          <FunctionUsageBreakdown
            team={team}
            usageByProject={visibleProjects}
            metricsByDeployment={metricsByFunction}
            metric={metric}
          />
        ) : (
          <UsageDataNotAvailable entity={`Breakdown by ${metric.name}`} />
        )}
      </div>
    </TeamUsageSection>
  );
}

type UsageInProject = {
  key: string;
  projectId: number;
  rows: FunctionMetricsRow[];
  total: number;
};

function useUsageByProject(
  callsByDeployment: FunctionMetricsRow[] | undefined,
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
  metricsByDeployment: FunctionMetricsRow[];
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
  rows: FunctionMetricsRow[];
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
            <SegmentedControl
              options={[
                { label: "Document Size", value: "size" },
                { label: "Document Count", value: "count" },
              ]}
              value={activeTab}
              onChange={(v) => {
                setActiveTab(v);
                setSelectedDate(null);
              }}
            />
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
                  isGauge
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
                  isGauge
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
                isGauge
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

export function FunctionCallsUsage({
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
          <h3 className="py-2">Function calls</h3>
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
                  isGauge
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
                isGauge
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
              selectedDate={selectedDate}
              setSelectedDate={setSelectedDate}
              isGauge
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
            isGauge
          />
        )}
      </div>
    </TeamUsageSection>
  );
}

function DeploymentCountUsageV2({
  team,
  dateRange,
  componentPrefix,
}: DetailSectionPropsV2) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<BusinessGroupBy>(
    "usageViewMode_businessDeploymentCount",
    "byType",
  );
  const viewMode = storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);

  const { data: deploymentsByClassAndRegion, error: deploymentsByClassError } =
    useDeploymentsByClassAndRegionV2(team.id, dateRange);

  const { data: deploymentCountByType, error: deploymentCountByTypeError } =
    useUsageTeamDeploymentCountByType(
      team.id,
      dateRange,
      null,
      componentPrefix,
    );

  const {
    data: deploymentCountDailyByProject,
    error: deploymentCountDailyByProjectError,
  } = useUsageTeamDeploymentCountPerDayByProject(
    team.id,
    dateRange,
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

  // Aggregate deployment data by class for the byDeploymentClass view
  const deploymentsByClass = useMemo(() => {
    if (deploymentsByClassAndRegion === undefined) return undefined;
    const grouped = groupBy(deploymentsByClassAndRegion, (row) => row.ds);
    return Object.entries(grouped).map(([ds, dayRows]) => {
      const classTotals = new Map<string, number>();
      for (const row of dayRows) {
        classTotals.set(
          row.deploymentClass,
          (classTotals.get(row.deploymentClass) || 0) + row.count,
        );
      }
      return {
        ds,
        metrics: Array.from(classTotals.entries()).map(([tag, value]) => ({
          tag,
          value,
        })),
      };
    });
  }, [deploymentsByClassAndRegion]);

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Deployments</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            options={BUSINESS_GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {viewMode === "byType" ? (
          deploymentCountByTypeError ? (
            <UsageDataError entity="Deployments" />
          ) : deploymentCountByType === undefined ? (
            <ChartLoading />
          ) : (
            <UsageStackedBarChart
              rows={deploymentCountByType}
              categories={deploymentTypeCategories}
              selectedDate={selectedDate}
              setSelectedDate={setSelectedDate}
              isGauge
            />
          )
        ) : viewMode === "byProject" ? (
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
              isGauge
            />
          )
        ) : deploymentsByClassError ? (
          <UsageDataError entity="Deployments" />
        ) : deploymentsByClass === undefined ? (
          <ChartLoading />
        ) : (
          <UsageStackedBarChart
            rows={deploymentsByClass}
            categories={DEPLOYMENT_CLASS_CATEGORIES}
            selectedDate={selectedDate}
            setSelectedDate={setSelectedDate}
            isGauge
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
                isGauge
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

// --- Business plan sections ---

function FunctionBreakdownSectionV2({
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
    useUsageTeamMetricsByFunctionV2(
      team.id,
      dateRange,
      projectId,
      componentPrefix,
    );

  const [functionBreakdownTab, setFunctionBreakdownTab] = useState(
    FUNCTION_BREAKDOWN_TABS_V2[0].name,
  );
  const metric =
    FUNCTION_BREAKDOWN_TABS_V2.find((t) => t.name === functionBreakdownTab) ??
    FUNCTION_BREAKDOWN_TABS_V2[0];
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
    functionBreakdownTab,
    setCurrentPage,
  ]);

  const functionBreakdownOptions = FUNCTION_BREAKDOWN_TABS_V2.map((tab) => ({
    label: tab.name.replace(/\b\w/g, (c) => c.toUpperCase()),
    value: tab.name,
  }));

  return (
    <TeamUsageSection
      stickyHeader
      header={
        <div className="flex w-full flex-col gap-2">
          <div className="flex items-center justify-between gap-4">
            <h3>Breakdown by function</h3>

            <SegmentedControl
              options={functionBreakdownOptions}
              value={functionBreakdownTab}
              onChange={setFunctionBreakdownTab}
            />
          </div>

          {totalPages > 1 && (
            <div className="flex justify-end">
              <PaginationControls
                currentPage={currentPage}
                totalPages={totalPages}
                onPageChange={setCurrentPage}
              />
            </div>
          )}
        </div>
      }
    >
      <div className="px-4">
        {metricsByFunctionError ? (
          <UsageDataError entity="Functions breakdown" />
        ) : !metricsByFunction ? (
          <ChartLoading />
        ) : (
          <FunctionUsageBreakdown
            team={team}
            usageByProject={visibleProjects}
            metricsByDeployment={metricsByFunction}
            metric={metric}
          />
        )}
      </div>
    </TeamUsageSection>
  );
}

type DetailSectionPropsV2 = {
  team: TeamResponse;
  dateRange: DateRange | null;
  projectId: number | null;
  componentPrefix: string | null;
  isBusinessPlan?: boolean;
};

// Helper to aggregate DailyPerTagMetricsByProjectAndClass to DailyPerTagMetrics
function aggregateTagByClassToByType(
  rows: DailyPerTagMetricsByProjectAndClass[] | undefined,
  projectId: number | null,
): DailyPerTagMetrics[] | undefined | null {
  if (rows === undefined) return undefined;
  const filtered =
    projectId === null
      ? rows
      : rows.filter((row) => row.projectId === projectId);
  const grouped = groupBy(filtered, (row) => row.ds);
  return Object.entries(grouped).map(([ds, dayRows]) => {
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

// Aggregate by-project-and-class data to by-project view (summing across deployment classes)
function aggregateTagByClassToByProject(
  rows: DailyPerTagMetricsByProjectAndClass[] | undefined,
): DailyPerTagMetricsByProject[] | undefined {
  if (rows === undefined) return undefined;
  const grouped = groupBy(rows, (row) => `${row.ds}-${row.projectId}`);
  return Object.entries(grouped).map(([, dayProjectRows]) => {
    const metricsMap = new Map<string, number>();
    for (const row of dayProjectRows) {
      for (const metric of row.metrics) {
        metricsMap.set(
          metric.tag,
          (metricsMap.get(metric.tag) || 0) + metric.value,
        );
      }
    }
    return {
      ds: dayProjectRows[0].ds,
      projectId: dayProjectRows[0].projectId,
      metrics: Array.from(metricsMap.entries()).map(([tag, value]) => ({
        tag,
        value,
      })),
    };
  });
}

// Aggregate by-project-and-class data to by-deployment-class view (summing across projects and tags)
function aggregateTagByProjectToByClass(
  rows: DailyPerTagMetricsByProjectAndClass[] | undefined,
): DailyPerTagMetrics[] | undefined {
  if (rows === undefined) return undefined;
  const grouped = groupBy(rows, (row) => row.ds);
  return Object.entries(grouped).map(([ds, dayRows]) => {
    const classTotals = new Map<string, number>();
    for (const row of dayRows) {
      const total = sumBy(row.metrics, (m) => m.value);
      classTotals.set(
        row.deploymentClass,
        (classTotals.get(row.deploymentClass) || 0) + total,
      );
    }
    return {
      ds,
      metrics: Array.from(classTotals.entries()).map(([tag, value]) => ({
        tag,
        value,
      })),
    };
  });
}

function FunctionCallsUsageV2({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: DetailSectionPropsV2) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<BusinessGroupBy>(
    "usageViewMode_businessFunctionCalls",
    "byType",
  );
  const viewMode = storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);
  const { data: callsByTagByProjectAndClass, error } =
    useFunctionCallsPerDayByProjectAndClassV2(
      team.id,
      dateRange,
      projectId,
      componentPrefix,
    );

  const callsByTag =
    viewMode === "byType"
      ? aggregateTagByClassToByType(callsByTagByProjectAndClass, null)
      : null;

  const callsByProject =
    viewMode === "byProject"
      ? aggregateTagByClassToByProject(callsByTagByProjectAndClass)
      : undefined;

  const callsByClass =
    viewMode === "byDeploymentClass"
      ? aggregateTagByProjectToByClass(callsByTagByProjectAndClass)
      : null;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Function calls</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            options={BUSINESS_GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {error ? (
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
            ) : viewMode === "byProject" ? (
              callsByProject === undefined ? (
                <ChartLoading />
              ) : (
                <UsageByProjectChart
                  rows={callsByProject}
                  team={team}
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : callsByClass === undefined ? (
              <ChartLoading />
            ) : callsByClass === null ? (
              <UsageChartUnavailable />
            ) : (
              <UsageStackedBarChart
                rows={callsByClass}
                categories={DEPLOYMENT_CLASS_CATEGORIES}
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

function ComputeUsageV2({
  team,
  dateRange,
  projectId,
  componentPrefix,
  isBusinessPlan = true,
}: DetailSectionPropsV2) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_businessCompute",
    "byType",
  );
  const viewMode = storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);
  const businessResult = useComputePerDayByProjectV2(
    team.id,
    dateRange,
    projectId,
    componentPrefix,
  );
  const selfServeResult = useComputePerDayByProjectSelfServeV2(
    team.id,
    dateRange,
    projectId,
    componentPrefix,
  );
  const { data: computeData, error } = isBusinessPlan
    ? businessResult
    : selfServeResult;

  const categories = isBusinessPlan
    ? COMPUTE_CATEGORIES
    : COMPUTE_CATEGORIES_SELF_SERVE;

  const daily =
    viewMode === "byType"
      ? aggregateByProjectToByType(computeData, null)
      : null;

  const title = isBusinessPlan ? "Compute" : "Action Compute";

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">{title}</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            options={GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {error ? (
          <UsageDataError entity={title} />
        ) : (
          <>
            {viewMode === "byType" ? (
              daily === undefined ? (
                <ChartLoading />
              ) : daily === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageStackedBarChart
                  rows={daily}
                  categories={categories}
                  quantityType="actionCompute"
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : computeData === undefined ? (
              <ChartLoading />
            ) : (
              <UsageByProjectChart
                rows={computeData}
                team={team}
                selectedDate={selectedDate}
                setSelectedDate={setSelectedDate}
                quantityType="actionCompute"
              />
            )}
          </>
        )}
      </div>
    </TeamUsageSection>
  );
}

function DatabaseStorageUsageV2({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: DetailSectionPropsV2) {
  const [storedViewMode, setViewMode] =
    useGlobalLocalStorage<BusinessDatabaseGroupBy>(
      "usageViewMode_businessDatabaseStorage",
      "byTable",
    );
  const viewMode = storedViewMode;

  const [activeTab, setActiveTab] = useState<"size" | "count">("size");
  const [selectedDate, setSelectedDate] = useState<number | null>(null);
  const { data: dataByProjectAndClass, error: storageError } =
    useDatabaseStoragePerDayByProjectAndClassV2(
      team.id,
      dateRange,
      projectId,
      componentPrefix,
    );

  const { data: databaseStorageByTable, error: databaseStorageByTableError } =
    useDatabaseStoragePerDayByTableV2(
      team.id,
      dateRange,
      projectId,
      componentPrefix,
    );

  const { data: documentsCountByProject, error: documentsCountByProjectError } =
    useUsageTeamDocumentsPerDayByProject(team.id, dateRange, componentPrefix);

  const { data: documentsCountByTable, error: documentsCountByTableError } =
    useDocumentCountPerDayByTableV2(
      team.id,
      dateRange,
      projectId,
      componentPrefix,
    );

  const daily =
    viewMode === "byType"
      ? aggregateTagByClassToByType(dataByProjectAndClass, null)
      : null;

  const dailyByProject =
    viewMode === "byProject"
      ? aggregateTagByClassToByProject(dataByProjectAndClass)
      : undefined;

  const dailyByClass =
    viewMode === "byDeploymentClass"
      ? aggregateTagByProjectToByClass(dataByProjectAndClass)
      : null;

  const documentsCount =
    viewMode === "byType"
      ? aggregateSimpleByProjectToByType(documentsCountByProject, null)
      : null;

  const hasError =
    activeTab === "size"
      ? storageError || databaseStorageByTableError
      : documentsCountByProjectError || documentsCountByTableError;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Database Storage</h3>
          <div className="flex flex-wrap items-center gap-3">
            <SegmentedControl
              options={[
                { label: "Document Size", value: "size" },
                { label: "Document Count", value: "count" },
              ]}
              value={activeTab}
              onChange={(v) => {
                setActiveTab(v);
                setSelectedDate(null);
                if (v === "count" && viewMode === "byDeploymentClass") {
                  setViewMode("byType");
                }
              }}
            />
            <GroupBySelector
              value={viewMode}
              onChange={setViewMode}
              options={
                activeTab === "count"
                  ? DATABASE_GROUP_BY_OPTIONS
                  : BUSINESS_DATABASE_GROUP_BY_OPTIONS
              }
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
              daily === undefined ? (
                <ChartLoading />
              ) : daily === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageStackedBarChart
                  rows={daily}
                  categories={DATABASE_STORAGE_CATEGORIES}
                  quantityType="storage"
                  isGauge
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
                  isGauge
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : viewMode === "byProject" ? (
              dailyByProject === undefined ? (
                <ChartLoading />
              ) : (
                <UsageByProjectChart
                  rows={dailyByProject}
                  team={team}
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                  quantityType="storage"
                  isGauge
                />
              )
            ) : dailyByClass === undefined ? (
              <ChartLoading />
            ) : dailyByClass === null ? (
              <UsageChartUnavailable />
            ) : (
              <UsageStackedBarChart
                rows={dailyByClass}
                categories={DEPLOYMENT_CLASS_CATEGORIES}
                quantityType="storage"
                isGauge
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

function DatabaseIOUsageV2({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: DetailSectionPropsV2) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<BusinessGroupBy>(
    "usageViewMode_businessDatabaseIO",
    "byType",
  );
  const viewMode = storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);
  const { data: dataByProjectAndClass, error } =
    useDatabaseIOPerDayByProjectAndClassV2(
      team.id,
      dateRange,
      projectId,
      componentPrefix,
    );

  const daily =
    viewMode === "byType"
      ? aggregateTagByClassToByType(dataByProjectAndClass, null)
      : null;

  const dailyByProject =
    viewMode === "byProject"
      ? aggregateTagByClassToByProject(dataByProjectAndClass)
      : undefined;

  const dailyByClass =
    viewMode === "byDeploymentClass"
      ? aggregateTagByProjectToByClass(dataByProjectAndClass)
      : null;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Database I/O</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            options={BUSINESS_GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {error ? (
          <UsageDataError entity="Database I/O" />
        ) : (
          <>
            {viewMode === "byType" ? (
              daily === undefined ? (
                <ChartLoading />
              ) : daily === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageStackedBarChart
                  rows={daily}
                  categories={DATABASE_IO_CATEGORIES}
                  quantityType="storage"
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : viewMode === "byProject" ? (
              dailyByProject === undefined ? (
                <ChartLoading />
              ) : (
                <UsageByProjectChart
                  rows={dailyByProject}
                  team={team}
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                  quantityType="storage"
                />
              )
            ) : dailyByClass === undefined ? (
              <ChartLoading />
            ) : dailyByClass === null ? (
              <UsageChartUnavailable />
            ) : (
              <UsageStackedBarChart
                rows={dailyByClass}
                categories={DEPLOYMENT_CLASS_CATEGORIES}
                quantityType="storage"
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

function SearchStorageUsageV2({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: DetailSectionPropsV2) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_businessSearchStorage",
    "byType",
  );
  const viewMode = storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);
  const { data, error } = useSearchStoragePerDayByProjectV2(
    team.id,
    dateRange,
    projectId,
    componentPrefix,
  );

  const daily =
    viewMode === "byType" ? aggregateByProjectToByType(data, null) : null;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Search Storage</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            options={GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {error ? (
          <UsageDataError entity="Search storage" />
        ) : (
          <>
            {viewMode === "byType" ? (
              daily === undefined ? (
                <ChartLoading />
              ) : daily === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageStackedBarChart
                  rows={daily}
                  categories={SEARCH_STORAGE_CATEGORIES}
                  quantityType="storage"
                  isGauge
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : data === undefined ? (
              <ChartLoading />
            ) : (
              <UsageByProjectChart
                rows={data}
                team={team}
                selectedDate={selectedDate}
                setSelectedDate={setSelectedDate}
                quantityType="storage"
                isGauge
              />
            )}
          </>
        )}
      </div>
    </TeamUsageSection>
  );
}

function FileStorageUsageV2({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: DetailSectionPropsV2) {
  const [selectedDate, setSelectedDate] = useState<number | null>(null);
  const { data, error } = useFileStoragePerDayByProjectV2(
    team.id,
    dateRange,
    projectId,
    componentPrefix,
  );

  return (
    <TeamUsageSection header={<h3 className="py-2">File Storage</h3>}>
      <div className="px-4">
        {error ? (
          <UsageDataError entity="File storage" />
        ) : data === undefined ? (
          <ChartLoading />
        ) : (
          <UsageByProjectChart
            rows={data}
            team={team}
            selectedDate={selectedDate}
            setSelectedDate={setSelectedDate}
            quantityType="storage"
            isGauge
          />
        )}
      </div>
    </TeamUsageSection>
  );
}

function DataEgressUsageV2({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: DetailSectionPropsV2) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_businessDataEgress",
    "byType",
  );
  const viewMode = storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);
  const { data, error } = useDataEgressPerDayByProjectV2(
    team.id,
    dateRange,
    projectId,
    componentPrefix,
  );

  const categories = DATA_EGRESS_CATEGORIES;
  const categoryRenames = DATA_EGRESS_CATEGORY_RENAMES;

  const daily =
    viewMode === "byType" ? aggregateByProjectToByType(data, null) : null;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Data Egress</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            options={GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {error ? (
          <UsageDataError entity="Data egress" />
        ) : (
          <>
            {viewMode === "byType" ? (
              daily === undefined ? (
                <ChartLoading />
              ) : daily === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageStackedBarChart
                  rows={daily}
                  categories={categories}
                  categoryRenames={categoryRenames}
                  quantityType="storage"
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : data === undefined ? (
              <ChartLoading />
            ) : (
              <UsageByProjectChart
                rows={data}
                team={team}
                selectedDate={selectedDate}
                setSelectedDate={setSelectedDate}
                quantityType="storage"
              />
            )}
          </>
        )}
      </div>
    </TeamUsageSection>
  );
}

function SearchQueriesUsageV2({
  team,
  dateRange,
  projectId,
  componentPrefix,
}: DetailSectionPropsV2) {
  const [storedViewMode, setViewMode] = useGlobalLocalStorage<GroupBy>(
    "usageViewMode_businessSearchQueries",
    "byType",
  );
  const viewMode = storedViewMode;

  const [selectedDate, setSelectedDate] = useState<number | null>(null);
  const { data, error } = useSearchQueriesPerDayByProjectV2(
    team.id,
    dateRange,
    projectId,
    componentPrefix,
  );

  const daily =
    viewMode === "byType" ? aggregateByProjectToByType(data, null) : null;

  const dailyByProject = viewMode === "byProject" ? data : undefined;

  return (
    <TeamUsageSection
      header={
        <>
          <h3 className="py-2">Search Queries</h3>
          <GroupBySelector
            value={viewMode}
            onChange={setViewMode}
            options={GROUP_BY_OPTIONS}
          />
        </>
      }
    >
      <div className="px-4">
        {error ? (
          <UsageDataError entity="Search queries" />
        ) : (
          <>
            {viewMode === "byType" ? (
              daily === undefined ? (
                <ChartLoading />
              ) : daily === null ? (
                <UsageChartUnavailable />
              ) : (
                <UsageStackedBarChart
                  rows={daily}
                  categories={SEARCH_QUERIES_CATEGORIES}
                  quantityType="textSearch"
                  selectedDate={selectedDate}
                  setSelectedDate={setSelectedDate}
                />
              )
            ) : dailyByProject === undefined ? (
              <ChartLoading />
            ) : (
              <UsageByProjectChart
                rows={dailyByProject}
                quantityType="textSearch"
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
