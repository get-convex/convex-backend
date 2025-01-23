import {
  calcBuckets,
  useDeploymentAuthHeader,
  useDeploymentUrl,
  useCurrentOpenFunction,
  cacheHitPercentage,
  latencyPercentiles,
  UdfMetric,
  udfRate,
} from "dashboard-common";
import { format } from "date-fns";
import { SingleGraph } from "./SingleGraph";

export function PerformanceGraphs() {
  const currentOpenFunction = useCurrentOpenFunction();
  const deploymentUrl = useDeploymentUrl();
  const authHeader = useDeploymentAuthHeader();

  if (!currentOpenFunction) {
    return null;
  }

  const file = currentOpenFunction!;
  const lineColors = [
    "rgb(var(--chart-line-1))",
    "rgb(var(--chart-line-2))",
    "rgb(var(--chart-line-3))",
    "rgb(var(--chart-line-4))",
  ];

  function genErrorOrInvocationFunc(
    udfMetric: UdfMetric,
    name: string,
    color: string,
  ) {
    return async function genChartData(start: Date, end: Date) {
      const { startTime, endTime, numBuckets, timeMultiplier, formatTime } =
        calcBuckets(start, end);
      const buckets = await udfRate({
        deploymentUrl,
        udfIdentifier: file.displayName,
        componentPath: file.componentPath ?? undefined,
        udfType: file.udfType,
        metric: udfMetric,
        start: startTime,
        end: endTime,
        numBuckets,
        authHeader,
      });
      const data = buckets.map((value) =>
        value.metric
          ? {
              time: formatTime(value.time),
              metric: value.metric * timeMultiplier,
            }
          : {
              time: formatTime(value.time),
              metric: 0,
            },
      );
      return Promise.resolve({
        data,
        xAxisKey: "time",
        lineKeys: [
          {
            key: "metric",
            name,
            color,
          },
        ],
      });
    };
  }

  const calcCacheHitPercentage = async (start: Date, end: Date) => {
    const buckets = await cacheHitPercentage({
      deploymentUrl,
      udfIdentifier: file.displayName,
      componentPath: file.componentPath ?? undefined,
      udfType: file.udfType,
      start,
      end,
      numBuckets: 60,
      authHeader,
    });
    const data = buckets.map((value) =>
      value.metric
        ? {
            time: format(value.time, "hh:mm a"),
            metric: Math.round((value.metric + Number.EPSILON) * 100) / 100,
          }
        : {
            time: format(value.time, "hh:mm a"),
            metric: 0,
          },
    );
    return Promise.resolve({
      data,
      xAxisKey: "time",
      lineKeys: [
        {
          key: "metric",
          name: "%",
          color: "rgb(var(--chart-line-1))",
        },
      ],
    });
  };

  const calcLatencyPercentiles = async (start: Date, end: Date) => {
    const mapPercentileToBuckets = await latencyPercentiles({
      deploymentUrl,
      udfIdentifier: file.displayName,
      componentPath: file.componentPath ?? undefined,
      udfType: file.udfType,
      percentiles: [50, 90, 95, 99],
      start,
      end,
      numBuckets: 60,
      authHeader,
    });
    const data = [];
    const lineKeys = [];
    const percentiles = [...mapPercentileToBuckets.keys()];
    const xAxisKey = "time";
    // eslint-disable-next-line no-restricted-syntax
    for (const [i, bucket] of mapPercentileToBuckets
      .get(percentiles[0])!
      .entries()) {
      const dataPoint: any = {};
      dataPoint[xAxisKey] = format(bucket.time, "h:mm a");
      // eslint-disable-next-line no-restricted-syntax
      for (const percentile of percentiles) {
        const { metric } = mapPercentileToBuckets.get(percentile)![i];
        dataPoint[`p${percentile}`] = metric
          ? Math.round((metric + Number.EPSILON) * 100000) / 100
          : 0; // convert to ms
      }
      data.push(dataPoint);
    }
    // eslint-disable-next-line no-restricted-syntax
    for (const [i, percentile] of percentiles.entries()) {
      const pstring = `p${percentile}`;
      const lineKey = {
        key: pstring,
        name: `ms ${pstring}`,
        color: lineColors[i],
      };
      lineKeys.push(lineKey);
    }
    return Promise.resolve({
      data,
      xAxisKey,
      lineKeys,
    });
  };

  return (
    <div
      className="grid gap-2"
      style={{
        gridTemplateColumns: "repeat(auto-fit, minmax(24rem, 1fr))",
      }}
    >
      <SingleGraph
        title="Function Calls"
        dataSource={genErrorOrInvocationFunc(
          "invocations",
          " function calls",
          "rgb(var(--chart-line-1))",
        )}
        syncId="fnMetrics"
      />
      <SingleGraph
        title="Errors"
        dataSource={genErrorOrInvocationFunc(
          "errors",
          " errors",
          "rgb(var(--chart-line-4))",
        )}
        syncId="fnMetrics"
      />
      <SingleGraph
        title="Execution Time"
        dataSource={calcLatencyPercentiles}
        syncId="fnMetrics"
      />
      {currentOpenFunction.udfType === "Query" && (
        <SingleGraph
          title="Cache Hit Rate"
          dataSource={calcCacheHitPercentage}
          syncId="fnMetrics"
        />
      )}
    </div>
  );
}
