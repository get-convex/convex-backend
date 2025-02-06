import {
  InsightsSummaryData,
  useBytesReadAverageByHour,
  useBytesReadCountByHour,
  useDocumentsReadAverageByHour,
  useDocumentsReadCountByHour,
  useOCCByHour,
} from "api/insights";

import { Sparklines, SparklinesCurve } from "react-sparklines";
import { LoadingTransition } from "dashboard-common/elements/Loading";

export function SparklineForInsight({
  insight,
}: {
  insight: InsightsSummaryData;
}) {
  switch (insight.kind) {
    case "bytesReadAverageThreshold":
      return <BytesReadAverageSparkline insight={insight} />;
    case "bytesReadCountThreshold":
      return <BytesReadCountSparkline insight={insight} />;
    case "docsReadAverageThreshold":
      return <DocumentsReadAverageSparkline insight={insight} />;
    case "docsReadCountThreshold":
      return <DocumentsReadCountSparkline insight={insight} />;
    case "occFailedPermanently":
    case "occRetried":
      return <OCCSparkline insight={insight} />;
    default: {
      const _exhaustiveCheck: never = insight;
      return null;
    }
  }
}

function Sparkline({
  data,
  height = 36,
  color = "rgb(var(--chart-line-1))",
  min = 0,
  max,
}: {
  data?: number[];
  height?: number;
  color?: string;
  min?: number;
  max?: number;
}) {
  return (
    <LoadingTransition
      loadingProps={{
        fullHeight: false,
        className: "h-[36px] w-60",
      }}
    >
      {data && (
        <Sparklines data={data} height={height} margin={0} min={min} max={max}>
          <SparklinesCurve color={color} />
        </Sparklines>
      )}
    </LoadingTransition>
  );
}

function BytesReadAverageSparkline({
  insight,
}: {
  insight: InsightsSummaryData & { kind: "bytesReadAverageThreshold" };
}) {
  const bytesReadAverageByHour = useBytesReadAverageByHour({
    functionId: insight.functionId,
    componentPath: insight.componentPath,
  });
  return (
    <Sparkline
      data={bytesReadAverageByHour?.map((b) => b.avg)}
      // 8 MB is the max data read limit
      max={8 * 1024 * 1024}
    />
  );
}

function BytesReadCountSparkline({
  insight,
}: {
  insight: InsightsSummaryData & { kind: "bytesReadCountThreshold" };
}) {
  const bytesReadCountByHour = useBytesReadCountByHour({
    functionId: insight.functionId,
    componentPath: insight.componentPath,
  });
  return <Sparkline data={bytesReadCountByHour?.map((b) => b.count)} />;
}

function DocumentsReadAverageSparkline({
  insight,
}: {
  insight: InsightsSummaryData & { kind: "docsReadAverageThreshold" };
}) {
  const bytesReadAverageByHour = useDocumentsReadAverageByHour({
    functionId: insight.functionId,
    componentPath: insight.componentPath,
  });
  return (
    <Sparkline
      data={bytesReadAverageByHour?.map((b) => b.avg)}
      // 8 MB is the max data read limit
      max={8 * 1024 * 1024}
    />
  );
}

function DocumentsReadCountSparkline({
  insight,
}: {
  insight: InsightsSummaryData & { kind: "docsReadCountThreshold" };
}) {
  const bytesReadCountByHour = useDocumentsReadCountByHour({
    functionId: insight.functionId,
    componentPath: insight.componentPath,
  });
  return <Sparkline data={bytesReadCountByHour?.map((b) => b.count)} />;
}

function OCCSparkline({
  insight,
}: {
  insight: InsightsSummaryData & {
    kind: "occFailedPermanently" | "occRetried";
  };
}) {
  const occByHour = useOCCByHour({
    functionId: insight.functionId,
    componentPath: insight.componentPath,
    tableName: insight.occTableName,
    permanentFailure: insight.kind === "occFailedPermanently",
  });

  return <Sparkline data={occByHour?.map((o) => o.occCalls)} />;
}
