import { Button } from "@ui/Button";
import { Sheet } from "@ui/Sheet";
import { Tooltip } from "@ui/Tooltip";
import { Spinner } from "@ui/Spinner";
import { Callout } from "@ui/Callout";
import { formatBytes, formatNumberCompact } from "@common/lib/format";
import { UsageSummaryRow } from "hooks/usageMetrics";
import { formatQuantity } from "./lib/formatQuantity";
import { ReactNode } from "react";
import { TeamEntitlementsResponse } from "generatedApi";
import {
  QuestionMarkCircledIcon,
  CrossCircledIcon,
  ChevronRightIcon,
  InfoCircledIcon,
} from "@radix-ui/react-icons";
import { cn } from "@ui/cn";
import { useRouter } from "next/router";
import { Donut } from "@ui/Donut";

const BUSINESS_METRIC_TO_SECTION: Record<string, string> = {
  functionCalls: "functionCalls",
  compute: "compute",
  databaseStorage: "databaseStorage",
  databaseIO: "databaseIO",
  fileStorage: "filesStorage",
  searchStorage: "searchStorage",
  searchQueries: "searchQueries",
  dataEgress: "dataEgress",
  deploymentCount: "deployments",
};

const SELF_SERVE_METRIC_TO_SECTION: Record<string, string> = {
  functionCalls: "functionCalls",
  actionCompute: "actionCompute",
  databaseStorage: "databaseStorage",
  databaseIO: "databaseIO",
  fileStorage: "filesStorage",
  searchStorage: "searchStorage",
  searchQueries: "searchQueries",
  dataEgress: "dataEgress",
  deploymentCount: "deployments",
};

type BusinessMetricKey =
  | keyof Omit<UsageSummaryRow, "deploymentClass" | "region">
  | "compute"
  | "actionCompute"
  | "deploymentCount";

type Section = {
  metric: BusinessMetricKey;
  // TODO: Remove string fallback once teamMaxSearchQueries is in generated types
  entitlement?: keyof TeamEntitlementsResponse | string;
  format: (value: number) => string;
  detail: string;
  title: string;
  suffix?: string;
  noOnDemand?: boolean;
};

const businessSections: Section[] = [
  {
    metric: "functionCalls",
    format: formatNumberCompact,
    detail: "The number of function calls across all deployments",
    title: "Function Calls",
  },
  {
    metric: "compute",
    format: (n: number) => formatQuantity(n, "actionCompute"),
    detail:
      "The total execution time of functions multiplied by their allocated RAM",
    title: "Compute",
  },
  {
    metric: "databaseStorage",
    format: formatBytes,
    detail: "The total size of all documents stored in your projects",
    title: "Database Storage",
  },
  {
    metric: "databaseIO",
    format: formatBytes,
    detail: "The amount of data read and written to the database",
    title: "Database I/O",
  },
  {
    metric: "fileStorage",
    format: formatBytes,
    detail: "The total size of all files stored in your projects",
    title: "File Storage",
  },
  {
    metric: "searchStorage",
    format: formatBytes,
    detail: "The total size of all text and vector search indexes stored",
    title: "Search Storage",
  },
  {
    metric: "searchQueries",
    format: (n: number) => formatQuantity(n, "textSearch"),
    detail: "The total query-GB of text and vector search queries",
    title: "Search Queries",
  },
  {
    metric: "dataEgress",
    format: formatBytes,
    detail:
      "The amount of data egressed via file serving, fetch requests, and log streaming",
    title: "Data Egress",
  },
  {
    metric: "deploymentCount",
    format: formatNumberCompact,
    detail: "The number of deployments across all projects",
    title: "Deployments",
  },
];

const selfServeSections: Section[] = [
  {
    metric: "functionCalls",
    entitlement: "teamMaxFunctionCalls",
    format: formatNumberCompact,
    detail:
      "The number of times any query, mutation, file access or other function was called",
    title: "Function Calls",
  },
  {
    metric: "actionCompute",
    entitlement: "teamMaxActionCompute",
    format: formatNumberCompact,
    suffix: "GB-hours",
    detail:
      "The execution time of all actions multiplied by their allocated amount of RAM",
    title: "Action Compute",
  },
  {
    metric: "databaseStorage",
    entitlement: "teamMaxDatabaseStorage",
    format: formatBytes,
    detail: "The total size of all documents stored in your projects",
    title: "Database Storage",
  },
  {
    metric: "databaseIO",
    entitlement: "teamMaxDatabaseBandwidth",
    format: formatBytes,
    detail: "The amount of data read and written to the database",
    title: "Database I/O",
  },
  {
    metric: "fileStorage",
    entitlement: "teamMaxFileStorage",
    format: formatBytes,
    detail: "The total size of all files stored in your projects",
    title: "File Storage",
  },
  {
    metric: "dataEgress",
    entitlement: "teamMaxFileBandwidth",
    format: formatBytes,
    detail:
      "The amount of data egressed by file serving, fetch calls in actions, log streams, and streaming export.",
    title: "Data Egress",
  },
  {
    metric: "searchStorage",
    entitlement: "teamMaxVectorStorage",
    format: formatBytes,
    detail: "The total size of all text and vector search indexes stored",
    title: "Search Storage",
  },
  {
    metric: "searchQueries",
    entitlement: "teamMaxSearchQueries",
    format: (n: number) => formatQuantity(n, "textSearch"),
    detail: "The total query-GB of text and vector search queries",
    title: "Search Queries",
  },
  {
    metric: "deploymentCount",
    entitlement: "maxDeployments",
    format: formatNumberCompact,
    detail: "The current number of deployments across all projects",
    title: "Deployments",
    noOnDemand: true,
  },
];

export function BusinessPlanSummary({
  summary,
  deploymentCount,
  error,
  isBusinessPlan = true,
  entitlements,
  hasSubscription = false,
  showEntitlements = false,
}: {
  summary?: UsageSummaryRow[];
  deploymentCount?: number;
  error?: any;
  isBusinessPlan?: boolean;
  entitlements?: TeamEntitlementsResponse;
  hasSubscription?: boolean;
  showEntitlements?: boolean;
}) {
  const router = useRouter();
  const activeSections = isBusinessPlan ? businessSections : selfServeSections;

  // Aggregate usage rows by summing each metric in activeSections.
  const aggregateRows = (rows: UsageSummaryRow[]) =>
    rows.reduce(
      (acc, row) => {
        for (const section of activeSections) {
          if (section.metric === "deploymentCount") continue;
          let value: number;
          if (section.metric === "compute") {
            value =
              row.queryMutationCompute +
              row.actionComputeUser +
              row.actionComputeNode;
          } else if (section.metric === "actionCompute") {
            value = row.actionComputeConvex + row.actionComputeNode;
          } else {
            value = row[section.metric];
          }
          acc[section.metric] = (acc[section.metric] || 0) + value;
        }
        return acc;
      },
      {} as Record<string, number>,
    );

  // Aggregate across deployment classes and regions
  const aggregated = summary ? aggregateRows(summary) : undefined;

  // Add deployment count from separate data source
  if (aggregated && deploymentCount !== undefined) {
    aggregated.deploymentCount = deploymentCount;
  }

  // For self-serve plans, aggregate only primary region (aws-us-east-1)
  // so that included limits only apply to US-hosted deployments.
  const primaryRegionAggregated =
    !isBusinessPlan && summary
      ? aggregateRows(summary.filter((row) => row.region === "aws-us-east-1"))
      : undefined;

  // Deployment count is not region-specific (it comes from a separate data
  // source), so its primary-region value is the full count.
  if (primaryRegionAggregated && deploymentCount !== undefined) {
    primaryRegionAggregated.deploymentCount = deploymentCount;
  }

  const sectionToRoute = isBusinessPlan
    ? BUSINESS_METRIC_TO_SECTION
    : SELF_SERVE_METRIC_TO_SECTION;

  const hasEuDeployments =
    !isBusinessPlan && summary?.some((s) => s.region !== "aws-us-east-1");

  return (
    <>
      {hasEuDeployments && (
        <Callout variant="instructions" className="flex items-start gap-2">
          <InfoCircledIcon className="mt-0.5 size-4 shrink-0" />
          <p>
            {hasSubscription ? (
              <>
                <span className="font-semibold">
                  EU region usage is billed on-demand.
                </span>{" "}
                Included plan limits only apply to US-hosted deployments. All
                usage on EU deployments is charged at on-demand rates, plus a
                30% regional surcharge.
              </>
            ) : (
              <>
                <span className="font-semibold">
                  EU region usage has no included limits on paid plans.
                </span>{" "}
                If you upgrade, included plan limits will only apply to
                US-hosted deployments. EU deployment usage will be billed
                on-demand at plan rates, plus a 30% regional surcharge.
              </>
            )}
          </p>
        </Callout>
      )}
      <Sheet
        className="animate-fadeInFromLoading overflow-hidden"
        padding={false}
      >
        <div className="flex flex-col gap-1 overflow-x-clip">
          <div
            className={cn(
              "grid items-center gap-2 rounded-t border-b px-4 py-2 text-sm text-content-secondary",
              hasSubscription
                ? "grid-cols-[4fr_3fr_2fr_auto] sm:grid-cols-[4fr_3fr_3fr_auto]"
                : "grid-cols-[5fr_4fr_auto]",
            )}
          >
            <div>Resource</div>
            <div>
              {hasSubscription ? (
                <div className="flex items-center gap-1">
                  Included{" "}
                  <Tooltip
                    tip="The amount of usage used within the included limits of your plan."
                    side="right"
                    className="hidden sm:block"
                  >
                    <QuestionMarkCircledIcon />
                  </Tooltip>
                </div>
              ) : (
                "Usage"
              )}
            </div>
            {hasSubscription && (
              <div className="flex items-center gap-1">
                On-demand{" "}
                <Tooltip
                  tip="Usage beyond your plan's included limits."
                  side="right"
                  className="hidden sm:block"
                >
                  <QuestionMarkCircledIcon />
                </Tooltip>
              </div>
            )}
            <span className="invisible flex items-center gap-1 text-xs">
              <span className="hidden whitespace-nowrap sm:inline">
                View breakdown by day
              </span>
              <ChevronRightIcon className="size-4" />
            </span>
          </div>
          {error ? (
            <PlanSummaryError />
          ) : !aggregated ? (
            <PlanSummaryLoading />
          ) : (
            activeSections.map((section, index) => {
              const sectionId = sectionToRoute[section.metric];
              const { section: _s, tab: _t, ...restQuery } = router.query;
              const linkQuery = sectionId
                ? { ...restQuery, section: sectionId }
                : restQuery;
              const linkHref = { pathname: router.pathname, query: linkQuery };

              const metric = aggregated[section.metric] ?? 0;
              const entitlement =
                section.entitlement && entitlements
                  ? ((entitlements as Record<string, unknown>)[
                      section.entitlement
                    ] as number | undefined)
                  : undefined;

              // For self-serve plans, only primary region (US) usage counts
              // toward included limits, matching the V1 behavior.
              const primaryRegionMetric = primaryRegionAggregated
                ? (primaryRegionAggregated[section.metric] ?? 0)
                : metric;
              const includedAmount =
                primaryRegionMetric !== undefined && entitlement !== undefined
                  ? Math.min(primaryRegionMetric, entitlement)
                  : undefined;
              const onDemandAmount =
                metric !== undefined && includedAmount !== undefined
                  ? metric - includedAmount
                  : undefined;

              const displayedUsage =
                hasSubscription &&
                !section.noOnDemand &&
                includedAmount !== undefined
                  ? includedAmount
                  : metric;

              return (
                <Button
                  key={index}
                  variant="unstyled"
                  onClick={() => {
                    void router.push(linkHref, undefined, { shallow: true });
                  }}
                  className={cn(
                    "group grid min-h-10 items-center gap-2 rounded-sm px-4 py-2 text-left transition-colors hover:bg-background-primary focus-visible:bg-background-primary focus-visible:outline-2 focus-visible:outline-border-selected",
                    hasSubscription
                      ? "grid-cols-[4fr_3fr_2fr_auto] sm:grid-cols-[4fr_3fr_3fr_auto]"
                      : "grid-cols-[5fr_4fr_auto]",
                  )}
                >
                  <div className="flex items-center gap-2">
                    {showEntitlements && entitlement !== undefined && (
                      <Tooltip
                        side="bottom"
                        tip={`Your team has used ${(100 * (displayedUsage / entitlement)).toFixed(2)}% of the included amount of ${section.title}.`}
                        className="flex animate-fadeInFromLoading items-center"
                      >
                        <Donut current={displayedUsage} max={entitlement} />
                      </Tooltip>
                    )}
                    <SectionLabel detail={section.detail}>
                      {section.title}
                    </SectionLabel>
                  </div>
                  <div className="animate-fadeInFromLoading">
                    <span>
                      {section.format(displayedUsage)}
                      {section.suffix &&
                        (!showEntitlements ? ` ${section.suffix}` : "")}
                    </span>
                    {showEntitlements && entitlement !== undefined && (
                      <span>
                        {" "}
                        / {section.format(entitlement)}
                        {section.suffix ? ` ${section.suffix}` : ""}
                      </span>
                    )}
                  </div>
                  {hasSubscription && (
                    <div className="animate-fadeInFromLoading">
                      {!section.noOnDemand &&
                        onDemandAmount !== undefined &&
                        onDemandAmount > 0 &&
                        `+${section.format(onDemandAmount)}${section.suffix ? ` ${section.suffix}` : ""}`}
                    </div>
                  )}
                  <span className="flex items-center gap-1 text-xs text-content-secondary">
                    <span className="hidden whitespace-nowrap opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100 sm:inline">
                      View breakdown by day
                    </span>
                    <ChevronRightIcon className="size-4" />
                  </span>
                </Button>
              );
            })
          )}
        </div>
      </Sheet>
    </>
  );
}

function PlanSummaryError() {
  return (
    <div className="flex h-56 flex-col items-center justify-center p-4 text-center">
      <CrossCircledIcon className="size-6 text-content-error" />
      <h5 className="mt-2">Error fetching Usage summary data</h5>
      <p className="mt-1 text-sm text-content-secondary">
        An error occurred while fetching usage summary data. Please try again
        later.
      </p>
    </div>
  );
}

function PlanSummaryLoading() {
  return (
    <div className="flex h-100 items-center justify-center p-4">
      <div className="flex items-center justify-center">
        <Spinner className="size-12" />
      </div>
    </div>
  );
}

function SectionLabel({
  detail,
  children,
}: {
  detail?: ReactNode;
  children: ReactNode;
}) {
  return (
    <p className="flex animate-fadeInFromLoading items-center text-sm">
      {children}
      {detail !== null && detail !== undefined ? (
        <Tooltip tip={detail} side="right" className="hidden sm:block">
          <QuestionMarkCircledIcon className="ml-1" />
        </Tooltip>
      ) : null}
    </p>
  );
}
