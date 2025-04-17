import { Insight } from "api/insights";

import { Sparklines, SparklinesCurve } from "react-sparklines";
import { LoadingTransition } from "@ui/Loading";

export function SparklineForInsight({ insight }: { insight: Insight }) {
  switch (insight.kind) {
    case "bytesReadLimit":
    case "bytesReadThreshold":
      return (
        <BytesReadCountSparkline
          insight={{
            ...insight,
            kind: insight.kind as "bytesReadLimit" | "bytesReadThreshold",
          }}
        />
      );
    case "docsReadLimit":
    case "docsReadThreshold":
      return (
        <DocumentsReadCountSparkline
          insight={{
            ...insight,
            kind: insight.kind as "docsReadLimit" | "docsReadThreshold",
          }}
        />
      );
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

function BytesReadCountSparkline({
  insight,
}: {
  insight: Insight & { kind: "bytesReadLimit" | "bytesReadThreshold" };
}) {
  return <Sparkline data={insight.details.hourlyCounts.map((b) => b.count)} />;
}

function DocumentsReadCountSparkline({
  insight,
}: {
  insight: Insight & { kind: "docsReadLimit" | "docsReadThreshold" };
}) {
  return <Sparkline data={insight.details.hourlyCounts.map((b) => b.count)} />;
}

function OCCSparkline({
  insight,
}: {
  insight: Insight & {
    kind: "occFailedPermanently" | "occRetried";
  };
}) {
  return <Sparkline data={insight.details.hourlyCounts.map((b) => b.count)} />;
}
