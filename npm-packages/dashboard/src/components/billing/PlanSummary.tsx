import { Sheet } from "dashboard-common/elements/Sheet";
import { Tooltip } from "dashboard-common/elements/Tooltip";
import { Loading } from "dashboard-common/elements/Loading";
import { formatBytes, formatNumberCompact } from "dashboard-common/lib/format";
import { UsageSummary } from "hooks/usageMetrics";
import { ReactNode } from "react";
import { TeamEntitlementsResponse } from "generatedApi";
import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";
import { cn } from "dashboard-common/lib/cn";
import Link from "next/link";

export function PlanSummary({
  teamSummary,
  entitlements,
  hasSubscription,
  showEntitlements,
}: {
  teamSummary?: UsageSummary;
  entitlements?: TeamEntitlementsResponse;
  hasSubscription: boolean;
  showEntitlements: boolean;
}) {
  return (
    <PlanSummaryForTeam
      teamSummary={teamSummary}
      entitlements={entitlements}
      hasSubscription={hasSubscription}
      showEntitlements={showEntitlements}
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
    | "vectorBandwidth";
  entitlement:
    | "teamMaxDatabaseStorage"
    | "teamMaxDatabaseBandwidth"
    | "teamMaxFunctionCalls"
    | "teamMaxActionCompute"
    | "teamMaxFileStorage"
    | "teamMaxFileBandwidth"
    | "teamMaxVectorStorage"
    | "teamMaxVectorBandwidth";
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
      "The number of times any query, mutation, file access or other function was called in the last month",
    title: "Function Calls",
  },
  {
    metric: "actionCompute",
    entitlement: "teamMaxActionCompute",
    format: formatNumberCompact,
    suffix: "GB-hours",
    detail:
      "The execution time of all actions multiplied by their allocated amount of RAM in the last month",
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
    detail: "The amount of data read and written in the last month",
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
    detail: "The amount of file data stored and read in the last month",
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
    detail: "The amount of data read and written for vector indexes last month",
    title: "Vector Bandwidth",
  },
];

export type PlanSummaryForTeamProps = {
  teamSummary?: UsageSummary;
  entitlements?: TeamEntitlementsResponse;
  showEntitlements: boolean;
  hasSubscription: boolean;
};

export function PlanSummaryForTeam({
  teamSummary,
  entitlements,
  hasSubscription,
  showEntitlements,
}: PlanSummaryForTeamProps) {
  return (
    <Sheet className="animate-fadeInFromLoading" padding={false}>
      <div className="flex flex-col gap-1 overflow-x-auto">
        <div
          className={cn(
            "grid items-center gap-2 rounded-t text-content-secondary text-sm px-4 py-2 border-b",
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
            metric={teamSummary ? teamSummary[section.metric] : undefined}
            entitlement={
              entitlements
                ? (entitlements[section.entitlement] ?? 0)
                : undefined
            }
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
}) {
  return (
    <Link
      href={`#${metricName}`}
      className={cn(
        "px-4 py-2 grid items-center gap-2 rounded group hover:bg-background-primary min-h-10",
        hasSubscription
          ? "grid-cols-[4fr_3fr_2fr] sm:grid-cols-[4fr_3fr_3fr]"
          : "grid-cols-[5fr_4fr]",
      )}
    >
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

export function Donut({
  current,
  max,
}: {
  current: number;
  max: number | null | undefined;
}) {
  if (max === null || max === undefined || max === 0) {
    return null;
  }
  // To draw a visible progress arc progress must be <1 and >=0.01
  const progress = Math.max(0.01, Math.min(current / max, 0.99999));
  const isOverHalf = progress >= 0.5;
  const radius = 13;
  const endAngle = 2 * Math.PI * progress - Math.PI / 2;
  const endX = radius * Math.cos(endAngle);
  const endY = radius * Math.sin(endAngle);
  const color = "stroke-util-accent";
  return (
    <div className="relative hidden sm:inline-block">
      <svg
        className="min-h-6 min-w-6"
        width="24"
        height="24"
        viewBox="-16 -16 32 32"
      >
        <circle r="16" className="fill-neutral-2 dark:fill-neutral-4" />
        <circle
          r="10"
          className="fill-background-secondary group-hover:fill-background-primary"
        />
        <path
          d={`M 0 -${radius}
            A ${radius} ${radius} 0 ${isOverHalf ? 1 : 0} 1 ${endX} ${endY}`}
          fill="transparent"
          className={color}
          strokeWidth="6"
        />
      </svg>
    </div>
  );
}
