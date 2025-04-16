import {
  InfoCircledIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import { InsightsSummaryData } from "api/insights";
import { Button } from "@ui/Button";
import { formatBytes, formatNumberCompact } from "@common/lib/format";
import Link from "next/link";

export function ProblemForInsight({
  insight,
  explain = false,
}: {
  insight: InsightsSummaryData;
  explain?: boolean;
}) {
  switch (insight.kind) {
    case "bytesReadAverageThreshold":
      return <BytesReadThresholdProblem insight={insight} explain={explain} />;
    case "bytesReadCountThreshold":
      return <BytesReadCountProblem insight={insight} explain={explain} />;
    case "docsReadAverageThreshold":
      return <DocsReadThresholdProblem insight={insight} explain={explain} />;
    case "docsReadCountThreshold":
      return <DocsReadCountProblem insight={insight} explain={explain} />;
    case "occFailedPermanently":
    case "occRetried":
      return <OCCProblem insight={insight} explain={explain} />;
    default: {
      const _exhaustiveCheck: never = insight;
      return null;
    }
  }
}

function OCCProblem({
  insight,
  explain,
}: {
  insight: InsightsSummaryData & {
    kind: "occFailedPermanently" | "occRetried";
  };
  explain: boolean;
}) {
  const isFailed = insight.kind === "occFailedPermanently";
  return (
    <div className="flex flex-col items-start gap-1">
      <span className="flex items-center gap-1 truncate">
        {isFailed ? "Failed" : "Retried"} due to write conflicts{" "}
        {explain && (
          <>
            in table{" "}
            <span className="font-semibold">{insight.occTableName}</span>
          </>
        )}
        {!explain && (
          <Button
            href="https://docs.convex.dev/error#1"
            tip={
              <>
                <Link
                  href="https://docs.convex.dev/error#1"
                  target="_blank"
                  className="underline"
                >
                  Learn more
                </Link>{" "}
                about write conflicts.
              </>
            }
            tipSide="right"
            variant="unstyled"
            target="_blank"
          >
            <QuestionMarkCircledIcon />
          </Button>
        )}
      </span>
      {!explain && (
        <span className="text-left text-xs text-content-secondary">
          {formatNumberCompact(insight.occCalls)} time
          {insight.occCalls === 1 ? "" : "s"} in{" "}
          {!insight.occTableName ? (
            "an unknown table"
          ) : (
            <>
              table{" "}
              <span className="font-semibold">{insight.occTableName}</span>
            </>
          )}
        </span>
      )}
      {explain && (
        <div className="my-2 flex max-w-prose items-start gap-2 text-pretty rounded border p-2 text-sm">
          <InfoCircledIcon className="mt-[3px] shrink-0" />
          <div className="space-y-2">
            <p>
              Write conflicts occur when two functions running in parallel make
              conflicting changes to the same table.{" "}
            </p>
            <p>
              Convex will attempt to retry mutations that fail this way, but
              will eventually fail permanently if the conflicts persist.
            </p>
            <p>
              <Link
                href="https://docs.convex.dev/error#1"
                className="text-content-link hover:underline"
                target="_blank"
              >
                Learn how to debug this Insight.
              </Link>
            </p>
          </div>
        </div>
      )}
    </div>
  );
}

function BytesReadCountProblem({
  insight,
  explain,
}: {
  insight: InsightsSummaryData & { kind: "bytesReadCountThreshold" };
  explain: boolean;
}) {
  return (
    <div className="flex flex-col items-start gap-1">
      <ReadLimitProblem explain={explain} kind="bytes" />
      {!explain && (
        <span className="text-xs text-content-secondary">
          {formatNumberCompact(insight.aboveThresholdCalls)} function call
          {insight.aboveThresholdCalls === 1 ? "" : "s"}
        </span>
      )}
    </div>
  );
}

function BytesReadThresholdProblem({
  insight,
  explain,
}: {
  insight: InsightsSummaryData & { kind: "bytesReadAverageThreshold" };
  explain: boolean;
}) {
  return (
    <div className="flex flex-col items-start gap-1">
      <ReadLimitProblem explain={explain} kind="bytes" />
      {!explain && (
        <span className="text-xs text-content-secondary">
          Avg. {formatBytes(insight.avgBytesRead)} per call{" "}
        </span>
      )}
    </div>
  );
}

export const documentsReadLimit = 32000;
export const megabytesReadLimit = 16;

const bytesLimitString = `${megabytesReadLimit} MB`;
const documentsLimitString = documentsReadLimit.toLocaleString();

function ReadLimitProblem({
  kind,
  explain,
}: {
  kind: "bytes" | "documents";
  explain: boolean;
}) {
  return (
    <div className="flex flex-col">
      <span className="flex items-center gap-1">
        Nearing {kind} read limit
        {!explain && (
          <Button
            href="https://stack.convex.dev/queries-that-scale"
            tip={`This function has been approaching or exceeding the Convex limit on ${kind} read. When a function exceeds the limit of ${kind === "bytes" ? bytesLimitString : documentsLimitString}, it will fail.`}
            tipSide="right"
            variant="unstyled"
            target="_blank"
          >
            <QuestionMarkCircledIcon />
          </Button>
        )}
      </span>
      {explain && (
        <div className="my-2 flex max-w-prose items-start gap-2 text-pretty rounded border p-2 text-sm">
          <InfoCircledIcon className="mt-[3px] shrink-0" />
          <div className="space-y-2">
            <p>
              This issue occurs when a function gets close to or exceeds the
              limit on {kind} read within a single transaction.{" "}
            </p>
            <p>
              When a function exceeds the{" "}
              <Link
                href="https://docs.convex.dev/production/state/limits#transactions"
                target="_blank"
                className="text-content-link hover:underline"
              >
                limit of{" "}
                {kind === "bytes" ? bytesLimitString : documentsLimitString}
              </Link>
              , it will fail.
            </p>
            <p>
              To limit the amount of data read by a function, consider{" "}
              <Link
                href="https://docs.convex.dev/database/indexes/indexes-and-query-perf"
                className="text-content-link hover:underline"
                target="_blank"
              >
                adding an index
              </Link>{" "}
              or{" "}
              <Link
                href="https://docs.convex.dev/database/pagination"
                className="text-content-link hover:underline"
                target="_blank"
              >
                implementing pagination
              </Link>
              .
            </p>
          </div>
        </div>
      )}
    </div>
  );
}

function DocsReadCountProblem({
  insight,
  explain,
}: {
  insight: InsightsSummaryData & { kind: "docsReadCountThreshold" };
  explain: boolean;
}) {
  return (
    <div className="flex flex-col items-start gap-1">
      <ReadLimitProblem explain={explain} kind="documents" />
      {!explain && (
        <span className="text-xs text-content-secondary">
          {formatNumberCompact(insight.aboveThresholdCalls)} function call
          {insight.aboveThresholdCalls === 1 ? "" : "s"}
        </span>
      )}
    </div>
  );
}

function DocsReadThresholdProblem({
  insight,
  explain,
}: {
  insight: InsightsSummaryData & { kind: "docsReadAverageThreshold" };
  explain: boolean;
}) {
  return (
    <div className="flex flex-col items-start gap-1">
      <ReadLimitProblem explain={explain} kind="documents" />
      {!explain && (
        <span className="text-xs text-content-secondary">
          Avg. {formatNumberCompact(insight.avgDocsRead)} per call{" "}
        </span>
      )}
    </div>
  );
}
