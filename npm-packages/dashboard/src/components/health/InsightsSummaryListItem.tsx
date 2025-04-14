import { InsightsSummaryData } from "api/insights";
import {
  CrossCircledIcon,
  ExclamationTriangleIcon,
} from "@radix-ui/react-icons";
import { Tooltip } from "@ui/Tooltip";
import { Button } from "@ui/Button";
import { FunctionNameOption } from "@common/elements/FunctionNameOption";
import { functionIdentifierValue } from "@common/lib/functions/generateFileTree";
import { useRouter } from "next/router";
import { SparklineForInsight } from "./SparklineForInsight";
import { ProblemForInsight } from "./ProblemForInsight";
import { useLogDeploymentEvent } from "../../hooks/deploymentApi";

export function InsightsSummaryListItem({
  insight,
}: {
  insight: InsightsSummaryData;
}) {
  const log = useLogDeploymentEvent();
  const { query, push } = useRouter();
  return (
    <Button
      className="flex w-full min-w-fit animate-fadeInFromLoading items-center gap-2 border-b p-2 text-xs last:border-b-0 hover:bg-background-tertiary"
      variant="unstyled"
      onClick={() => {
        void push(
          {
            pathname: "/t/[team]/[project]/[deploymentName]",
            query: {
              team: query.team,
              project: query.project,
              deploymentName: query.deploymentName,
              view: `insight:${insight.kind}:${insight.componentPath}:${insight.functionId}`,
              ...(query.lowInsightsThreshold
                ? { lowInsightsThreshold: query.lowInsightsThreshold }
                : {}),
            },
          },
          undefined,
          { shallow: true },
        );
        log("open insight", { kind: insight.kind });
      }}
    >
      <span className="w-20">
        {severityForInsightKind[insight.kind] === "error" ? (
          <Tooltip
            className="flex w-fit gap-1 rounded border bg-background-error p-1 text-xs text-content-error"
            tip="This insight is a critical problem and should be addressed soon."
            side="left"
          >
            <CrossCircledIcon /> Critical
          </Tooltip>
        ) : (
          <Tooltip
            className="flex w-fit gap-1 rounded border bg-background-warning p-1 text-xs text-content-warning"
            tip="This insight indicates a potential issue and should be investigated."
            side="left"
          >
            <ExclamationTriangleIcon /> Warning
          </Tooltip>
        )}
      </span>
      <div className="w-72 font-semibold">
        <FunctionNameOption
          label={functionIdentifierValue(
            insight.functionId,
            insight.componentPath ?? undefined,
          )}
          oneLine
        />
      </div>
      <div className="w-60">
        <ProblemForInsight insight={insight} />
      </div>
      <div className="w-60">
        <SparklineForInsight insight={insight} />
      </div>
    </Button>
  );
}

const severityForInsightKind: Record<
  InsightsSummaryData["kind"],
  "error" | "warning"
> = {
  bytesReadAverageThreshold: "error",
  bytesReadCountThreshold: "warning",
  docsReadAverageThreshold: "error",
  docsReadCountThreshold: "warning",
  occFailedPermanently: "error",
  occRetried: "warning",
};
