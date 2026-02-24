import { Button } from "@ui/Button";
import { Sheet } from "@ui/Sheet";
import { Tooltip } from "@ui/Tooltip";
import { Loading } from "@ui/Loading";
import { Spinner } from "@ui/Spinner";
import { formatBytes, formatNumberCompact } from "@common/lib/format";
import { UsageSummary } from "hooks/usageMetrics";
import { ReactNode } from "react";
import { GetTokenInfoResponse, TeamEntitlementsResponse } from "generatedApi";
import {
  QuestionMarkCircledIcon,
  CrossCircledIcon,
  ChevronRightIcon,
} from "@radix-ui/react-icons";
import { cn } from "@ui/cn";
import { useRouter } from "next/router";
import { Donut } from "@ui/Donut";

const METRIC_TO_SECTION: Record<string, string> = {
  functionCalls: "functionCalls",
  actionCompute: "actionCompute",
  databaseStorage: "databaseStorage",
  databaseBandwidth: "databaseBandwidth",
  fileStorage: "filesStorage",
  fileBandwidth: "filesBandwidth",
  vectorStorage: "vectorsStorage",
  vectorBandwidth: "vectorsBandwidth",
  deploymentCount: "deployments",
};

export function PlanSummary({
  chefTokenUsage,
  teamSummary,
  deploymentCount,
  entitlements,
  hasSubscription,
  showEntitlements,
  hasFilter,
  error,
}: {
  chefTokenUsage?: GetTokenInfoResponse;
  teamSummary?: UsageSummary[];
  deploymentCount?: number;
  entitlements?: TeamEntitlementsResponse;
  hasSubscription: boolean;
  showEntitlements: boolean;
  hasFilter: boolean;
  error?: any;
}) {
  return (
    <PlanSummaryForTeam
      chefTokenUsage={chefTokenUsage}
      teamSummary={teamSummary}
      deploymentCount={deploymentCount}
      entitlements={entitlements}
      hasSubscription={hasSubscription}
      showEntitlements={showEntitlements}
      hasFilter={hasFilter}
      error={error}
    />
  );
}

const sections: {
  metric:
    | "databaseStorage"
    | "databaseBandwidth"
    | "functionCalls"
    | "actionCompute"
    | "fileStorage"
    | "fileBandwidth"
    | "vectorStorage"
    | "vectorBandwidth"
    | "chefTokens"
    | "deploymentCount";
  entitlement:
    | "teamMaxDatabaseStorage"
    | "teamMaxDatabaseBandwidth"
    | "teamMaxFunctionCalls"
    | "teamMaxActionCompute"
    | "teamMaxFileStorage"
    | "teamMaxFileBandwidth"
    | "teamMaxVectorStorage"
    | "teamMaxVectorBandwidth"
    | "maxChefTokens"
    | "maxDeployments";
  format: (value: number) => string;
  detail: string;
  title: string;
  suffix?: string;
  noOnDemand?: boolean;
}[] = [
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
    detail: "The current total size of all documents stored in your projects",
    title: "Database Storage",
  },
  {
    metric: "databaseBandwidth",
    entitlement: "teamMaxDatabaseBandwidth",
    format: formatBytes,
    detail: "The amount of data read and written",
    title: "Database Bandwidth",
  },
  {
    metric: "fileStorage",
    entitlement: "teamMaxFileStorage",
    format: formatBytes,
    detail: "The current total size of all files stored in your projects",
    title: "File Storage",
  },
  {
    metric: "fileBandwidth",
    entitlement: "teamMaxFileBandwidth",
    format: formatBytes,
    detail: "The amount of file data stored and read",
    title: "File Bandwidth",
  },
  {
    metric: "vectorStorage",
    entitlement: "teamMaxVectorStorage",
    format: formatBytes,
    detail: "The current total size of all vectors stored in vector indexes",
    title: "Vector Storage",
  },
  {
    metric: "vectorBandwidth",
    entitlement: "teamMaxVectorBandwidth",
    format: formatBytes,
    detail: "The amount of data read and written for vector indexes",
    title: "Vector Bandwidth",
  },
  {
    metric: "deploymentCount",
    entitlement: "maxDeployments",
    format: formatNumberCompact,
    detail: "The current number of deployments across all projects",
    title: "Deployments",
    noOnDemand: true,
  },
  {
    metric: "chefTokens",
    entitlement: "maxChefTokens",
    format: (n: number) => `${formatNumberCompact(n)} Tokens`,
    detail: "The number of Chef tokens used",
    title: "Chef Tokens",
  },
];

export type PlanSummaryForTeamProps = {
  chefTokenUsage?: GetTokenInfoResponse;
  teamSummary?: UsageSummary[];
  deploymentCount?: number;
  entitlements?: TeamEntitlementsResponse;
  showEntitlements: boolean;
  hasSubscription: boolean;
  hasFilter: boolean;
  error?: any;
};

// Helper to aggregate usage metrics across regions
// aws-us-east-1 counts towards "Included", other regions go to "On-demand"
function aggregateRegionalMetric(
  teamSummary: UsageSummary[] | undefined,
  metricKey: keyof Omit<UsageSummary, "region">,
): { total: number; primaryRegion: number } | undefined {
  if (!teamSummary || teamSummary.length === 0) {
    return undefined;
  }

  const primaryRegionData = teamSummary.find(
    (s) => s.region === "aws-us-east-1",
  );
  const primaryRegion = primaryRegionData?.[metricKey] ?? 0;
  const total = teamSummary.reduce((sum, s) => sum + s[metricKey], 0);

  return { total, primaryRegion };
}

export function PlanSummaryForTeam({
  chefTokenUsage,
  teamSummary,
  deploymentCount,
  entitlements,
  hasSubscription,
  showEntitlements,
  hasFilter,
  error,
}: PlanSummaryForTeamProps) {
  return (
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
                  tip="The amount of usage used within the included limits of your plan. Built-in usage limits are only applied to deployments hosted in the US region."
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
                tip="The amount of usage used in addition to the included amount. On-demand usage incurs a surcharge based on the pricing of your plan."
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
        ) : !teamSummary ? (
          <PlanSummaryLoading />
        ) : (
          sections.map((section, index) => {
            let metric: number | undefined;
            let primaryRegionMetric: number | undefined;

            if (section.metric === "chefTokens") {
              metric = chefTokenUsage
                ? chefTokenUsage.centitokensUsed / 100
                : undefined;
              primaryRegionMetric = metric; // Chef tokens are not region-specific
            } else if (section.metric === "deploymentCount") {
              metric = deploymentCount;
              primaryRegionMetric = deploymentCount; // Deployment count is not region-specific
            } else {
              const aggregated = aggregateRegionalMetric(
                teamSummary,
                section.metric,
              );
              metric = aggregated?.total;
              primaryRegionMetric = aggregated?.primaryRegion;
            }

            return (
              <UsageSection
                key={index}
                metric={metric}
                primaryRegionMetric={primaryRegionMetric}
                entitlement={
                  section.metric === "chefTokens"
                    ? chefTokenUsage
                      ? chefTokenUsage.centitokensQuota / 100
                      : undefined
                    : entitlements
                      ? (entitlements[section.entitlement] ?? 0)
                      : undefined
                }
                isNotSubjectToFilter={
                  section.metric === "chefTokens" && hasFilter
                }
                hasSubscription={hasSubscription}
                metricName={section.metric}
                format={section.format}
                detail={section.detail}
                title={section.title}
                suffix={section.suffix}
                showEntitlements={showEntitlements}
                noOnDemand={section.noOnDemand}
              />
            );
          })
        )}
      </div>
    </Sheet>
  );
}

function PlanSummaryError() {
  return (
    <div className="flex h-56 flex-col items-center justify-center p-4 text-center">
      <CrossCircledIcon className="h-6 w-6 text-content-error" />
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
    <div className="flex h-[25rem] items-center justify-center p-4">
      <div className="flex items-center justify-center">
        <Spinner className="size-12" />
      </div>
    </div>
  );
}

export function UsageOverview(props: {
  metric?: number;
  primaryRegionMetric?: number;
  entitlement?: number;
  hasSubscription?: boolean;
  format: (value: number) => string;
  detail?: string;
  title?: string;
  suffix?: string;
  showEntitlements: boolean;
  noOnDemand?: boolean;
}) {
  return (
    <div className="mb-4 flex items-center gap-2">
      <UsageAmount {...props} />
    </div>
  );
}
function UsageAmount({
  metric,
  primaryRegionMetric,
  entitlement,
  hasSubscription = false,
  format,
  detail,
  title,
  suffix = "",
  showEntitlements,
  noOnDemand = false,
}: {
  metric?: number;
  primaryRegionMetric?: number;
  entitlement?: number;
  hasSubscription?: boolean;
  format: (value: number) => string;
  detail?: string;
  title?: string;
  suffix?: string;
  showEntitlements: boolean;
  noOnDemand?: boolean;
}) {
  // primaryRegionMetric is aws-us-east-1 usage, ONLY this counts for "Included"
  const includedMetric = primaryRegionMetric;
  const totalMetric = metric;

  // Calculate included and on-demand amounts
  // Included: min(aws-us-east-1 usage, entitlement)
  // On-demand: total - included
  const includedAmount =
    includedMetric !== undefined && entitlement !== undefined
      ? Math.min(includedMetric, entitlement)
      : undefined;
  const onDemandAmount =
    totalMetric !== undefined && includedAmount !== undefined
      ? totalMetric - includedAmount
      : undefined;

  return (
    <>
      <div className="flex items-center gap-2">
        {showEntitlements &&
          includedMetric !== undefined &&
          entitlement !== undefined && (
            <Tooltip
              side="bottom"
              tip={`Your team has used ${Math.floor(100 * (includedMetric / entitlement))}% of the included amount${title ? ` of ${title}` : ``}.`}
              className="flex animate-fadeInFromLoading items-center"
            >
              <Donut current={includedMetric} max={entitlement} />
            </Tooltip>
          )}
        {title && <SectionLabel detail={detail}>{title}</SectionLabel>}
      </div>
      {totalMetric === undefined || entitlement === undefined ? (
        <Loading />
      ) : (
        <Value
          limit={
            showEntitlements && !(noOnDemand && totalMetric > entitlement)
              ? format(entitlement) + (suffix ? ` ${suffix}` : "")
              : null
          }
        >
          {format(
            hasSubscription && !noOnDemand && includedAmount !== undefined
              ? includedAmount
              : totalMetric,
          )}
          {!showEntitlements && suffix ? ` ${suffix}` : ""}
        </Value>
      )}
      {hasSubscription &&
        (totalMetric === undefined || entitlement === undefined ? (
          <Loading />
        ) : (
          <Value>
            {!noOnDemand &&
              onDemandAmount !== undefined &&
              onDemandAmount > 0 &&
              `+${format(onDemandAmount)}${suffix ? ` ${suffix}` : ""}`}
          </Value>
        ))}
    </>
  );
}
function UsageSection({
  metric,
  primaryRegionMetric,
  metricName,
  entitlement,
  hasSubscription,
  format,
  detail,
  title,
  suffix = "",
  showEntitlements,
  isNotSubjectToFilter,
  noOnDemand = false,
}: {
  metric?: number;
  primaryRegionMetric?: number;
  metricName: string;
  entitlement?: number;
  hasSubscription: boolean;
  format: (value: number) => string;
  detail: string;
  title: string;
  suffix?: string;
  showEntitlements: boolean;
  isNotSubjectToFilter: boolean;
  noOnDemand?: boolean;
}) {
  const router = useRouter();
  const className = cn(
    "group grid min-h-10 items-center gap-2 rounded-sm px-4 py-2 text-left transition-colors focus-visible:outline-2 focus-visible:outline-border-selected",
    hasSubscription
      ? "grid-cols-[4fr_3fr_2fr_auto] sm:grid-cols-[4fr_3fr_3fr_auto]"
      : "grid-cols-[5fr_4fr_auto]",
    isNotSubjectToFilter
      ? "bg-stripes"
      : "hover:bg-background-primary focus-visible:bg-background-primary",
  );

  if (metricName === "chefTokens") {
    const content = (
      <div className={className}>
        <UsageAmount
          {...{
            metric,
            primaryRegionMetric,
            entitlement,
            hasSubscription,
            format,
            detail,
            title,
            suffix,
            showEntitlements,
            noOnDemand,
          }}
        />
        <span className="invisible flex items-center gap-1 text-xs">
          <span className="hidden whitespace-nowrap sm:inline">
            View breakdown by day
          </span>
          <ChevronRightIcon className="size-4" />
        </span>
      </div>
    );
    if (isNotSubjectToFilter) {
      return (
        <Tooltip
          tip="This metric does not support filtering by project or component"
          side="bottom"
          asChild
        >
          {content}
        </Tooltip>
      );
    }
    return content;
  }

  const section = METRIC_TO_SECTION[metricName];
  const { section: _s, tab: _t, ...restQuery } = router.query;
  const linkQuery = section
    ? {
        ...restQuery,
        section,
      }
    : restQuery;

  const linkHref = { pathname: router.pathname, query: linkQuery };

  return (
    <Button
      variant="unstyled"
      onClick={() => {
        void router.push(linkHref, undefined, { shallow: true });
      }}
      className={className}
    >
      <UsageAmount
        {...{
          metric,
          primaryRegionMetric,
          entitlement,
          hasSubscription,
          format,
          detail,
          title,
          suffix,
          showEntitlements,
          noOnDemand,
        }}
      />
      <span className="flex items-center gap-1 text-xs text-content-secondary">
        <span className="hidden whitespace-nowrap opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100 sm:inline">
          View breakdown by day
        </span>
        <ChevronRightIcon className="size-4" />
      </span>
    </Button>
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

function Value({
  limit,
  children,
}: {
  limit?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="animate-fadeInFromLoading">
      <span>{children}</span>
      {/* Wrapping in a span here is purposeful https://github.com/facebook/react/issues/11538#issuecomment-390386520 */}
      {limit !== null && limit !== undefined ? <span> / {limit}</span> : null}
    </div>
  );
}
