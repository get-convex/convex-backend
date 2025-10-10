import { Sheet } from "@ui/Sheet";
import { Tooltip } from "@ui/Tooltip";
import { Loading } from "@ui/Loading";
import { formatBytes, formatNumberCompact } from "@common/lib/format";
import { UsageSummary } from "hooks/usageMetrics";
import { ReactNode } from "react";
import { GetTokenInfoResponse, TeamEntitlementsResponse } from "generatedApi";
import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";
import { cn } from "@ui/cn";
import Link from "next/link";
import { Donut } from "@ui/Donut";

export function PlanSummary({
  chefTokenUsage,
  teamSummary,
  entitlements,
  hasSubscription,
  showEntitlements,
  hasFilter,
}: {
  chefTokenUsage?: GetTokenInfoResponse;
  teamSummary?: UsageSummary;
  entitlements?: TeamEntitlementsResponse;
  hasSubscription: boolean;
  showEntitlements: boolean;
  hasFilter: boolean;
}) {
  return (
    <PlanSummaryForTeam
      chefTokenUsage={chefTokenUsage}
      teamSummary={teamSummary}
      entitlements={entitlements}
      hasSubscription={hasSubscription}
      showEntitlements={showEntitlements}
      hasFilter={hasFilter}
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
    | "chefTokens";
  entitlement:
    | "teamMaxDatabaseStorage"
    | "teamMaxDatabaseBandwidth"
    | "teamMaxFunctionCalls"
    | "teamMaxActionCompute"
    | "teamMaxFileStorage"
    | "teamMaxFileBandwidth"
    | "teamMaxVectorStorage"
    | "teamMaxVectorBandwidth"
    | "maxChefTokens";
  format: (value: number) => string;
  detail: string;
  title: string;
  suffix?: string;
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
    metric: "chefTokens",
    entitlement: "maxChefTokens",
    format: (n: number) => `${formatNumberCompact(n)} Tokens`,
    detail: "The number of Chef tokens used",
    title: "Chef Tokens",
  },
];

export type PlanSummaryForTeamProps = {
  chefTokenUsage?: GetTokenInfoResponse;
  teamSummary?: UsageSummary;
  entitlements?: TeamEntitlementsResponse;
  showEntitlements: boolean;
  hasSubscription: boolean;
  hasFilter: boolean;
};

export function PlanSummaryForTeam({
  chefTokenUsage,
  teamSummary,
  entitlements,
  hasSubscription,
  showEntitlements,
  hasFilter,
}: PlanSummaryForTeamProps) {
  return (
    <Sheet
      className="animate-fadeInFromLoading overflow-hidden"
      padding={false}
    >
      <div className="flex flex-col gap-1 overflow-x-auto">
        <div
          className={cn(
            "grid items-center gap-2 rounded-t border-b px-4 py-2 text-sm text-content-secondary",
            hasSubscription
              ? "grid-cols-[4fr_3fr_2fr] sm:grid-cols-[4fr_3fr_3fr]"
              : "grid-cols-[5fr_4fr]",
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
                tip="The amount of usage used in addition to the included amount. On-demand usage incurs a surcharge based on the pricing of your plan."
                side="right"
                className="hidden sm:block"
              >
                <QuestionMarkCircledIcon />
              </Tooltip>
            </div>
          )}
        </div>
        {sections.map((section, index) => (
          <UsageSection
            key={index}
            metric={
              section.metric === "chefTokens"
                ? chefTokenUsage
                  ? chefTokenUsage.centitokensUsed / 100
                  : undefined
                : teamSummary
                  ? teamSummary[section.metric]
                  : undefined
            }
            entitlement={
              section.metric === "chefTokens"
                ? chefTokenUsage
                  ? chefTokenUsage.centitokensQuota / 100
                  : undefined
                : entitlements
                  ? (entitlements[section.entitlement] ?? 0)
                  : undefined
            }
            isNotSubjectToFilter={section.metric === "chefTokens" && hasFilter}
            hasSubscription={hasSubscription}
            metricName={section.metric}
            format={section.format}
            detail={section.detail}
            title={section.title}
            suffix={section.suffix}
            showEntitlements={showEntitlements}
          />
        ))}
      </div>
    </Sheet>
  );
}

export function UsageOverview(props: {
  metric?: number;
  entitlement?: number;
  hasSubscription?: boolean;
  format: (value: number) => string;
  detail?: string;
  title?: string;
  suffix?: string;
  showEntitlements: boolean;
}) {
  return (
    <div className="mb-4 flex items-center gap-2">
      <UsageAmount {...props} />
    </div>
  );
}
function UsageAmount({
  metric,
  entitlement,
  hasSubscription = false,
  format,
  detail,
  title,
  suffix = "",
  showEntitlements,
}: {
  metric?: number;
  entitlement?: number;
  hasSubscription?: boolean;
  format: (value: number) => string;
  detail?: string;
  title?: string;
  suffix?: string;
  showEntitlements: boolean;
}) {
  return (
    <>
      <div className="flex items-center gap-2">
        {showEntitlements &&
          (metric !== undefined && entitlement !== undefined ? (
            <Tooltip
              side="bottom"
              tip={`Your team has used ${Math.floor(100 * (metric / entitlement))}% of the included amount${title ? ` of ${title}` : ``}.`}
              className="flex animate-fadeInFromLoading items-center"
            >
              <Donut current={metric} max={entitlement} />
            </Tooltip>
          ) : (
            <Loading className="h-6 w-6" />
          ))}
        {title && <SectionLabel detail={detail}>{title}</SectionLabel>}
      </div>
      {metric === undefined || entitlement === undefined ? (
        <Loading />
      ) : (
        <Value
          limit={
            showEntitlements
              ? format(entitlement) + (suffix ? ` ${suffix}` : "")
              : null
          }
        >
          {format(hasSubscription ? Math.min(metric, entitlement) : metric)}
          {!showEntitlements && suffix ? ` ${suffix}` : ""}
        </Value>
      )}
      {hasSubscription &&
        (metric === undefined || entitlement === undefined ? (
          <Loading />
        ) : (
          <Value>
            {metric > entitlement &&
              `+${format(metric - entitlement)}${suffix ? ` ${suffix}` : ""}`}
          </Value>
        ))}
    </>
  );
}
function UsageSection({
  metric,
  metricName,
  entitlement,
  hasSubscription,
  format,
  detail,
  title,
  suffix = "",
  showEntitlements,
  isNotSubjectToFilter,
}: {
  metric?: number;
  metricName: string;
  entitlement?: number;
  hasSubscription: boolean;
  format: (value: number) => string;
  detail: string;
  title: string;
  suffix?: string;
  showEntitlements: boolean;
  isNotSubjectToFilter: boolean;
}) {
  const className = cn(
    "group grid min-h-10 items-center gap-2 rounded-sm px-4 py-2 transition-colors",
    hasSubscription
      ? "grid-cols-[4fr_3fr_2fr] sm:grid-cols-[4fr_3fr_3fr]"
      : "grid-cols-[5fr_4fr]",
    isNotSubjectToFilter ? "bg-stripes" : "hover:bg-background-primary",
  );

  if (metricName === "chefTokens") {
    const content = (
      <div className={className}>
        <UsageAmount
          {...{
            metric,
            entitlement,
            hasSubscription,
            format,
            detail,
            title,
            suffix,
            showEntitlements,
          }}
        />
      </div>
    );
    if (isNotSubjectToFilter) {
      return (
        <Tooltip
          tip="This metric does not support filtering by project or component"
          side="bottom"
          wrapsButton
        >
          {content}
        </Tooltip>
      );
    }
    return content;
  }

  return (
    <Link href={`#${metricName}`} className={className}>
      <UsageAmount
        {...{
          metric,
          entitlement,
          hasSubscription,
          format,
          detail,
          title,
          suffix,
          showEntitlements,
        }}
      />
    </Link>
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
