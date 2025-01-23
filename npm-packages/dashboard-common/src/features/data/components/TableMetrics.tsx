import {
  ChartData,
  useDeploymentAuthHeader,
  useDeploymentUrl,
} from "dashboard-common";
import React from "react";
import { ChartModal } from "../../../elements/ChartModal";
import { calcBuckets } from "../../../lib/charts/buckets";
import { TableMetric, tableRate } from "../../../lib/appMetrics";

const useChartData =
  (
    deploymentUrl: string,
    tableName: string,
    metric: TableMetric,
    authHeader: string,
    name: string,
    color = "rgb(var(--chart-line-1))",
  ) =>
  async (start: Date, end: Date): Promise<ChartData> => {
    const { startTime, endTime, numBuckets, timeMultiplier, formatTime } =
      calcBuckets(start, end);

    const buckets = await tableRate(
      deploymentUrl,
      tableName,
      metric,
      startTime,
      endTime,
      numBuckets,
      authHeader,
    );

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
    const xAxisKey = "time";
    const lineKeys = [
      {
        key: "metric",
        name,
        color,
      },
    ];
    return { data, xAxisKey, lineKeys };
  };

export function TableMetrics({
  tableName,
  onClose,
}: {
  tableName: string;
  onClose: () => void;
}) {
  const deploymentUrl = useDeploymentUrl();
  const authHeader = useDeploymentAuthHeader();

  const readsSource = useChartData(
    deploymentUrl,
    tableName,
    "rowsRead",
    authHeader,
    " reads",
  );

  const writesSource = useChartData(
    deploymentUrl,
    tableName,
    "rowsWritten",
    authHeader,
    " writes",
    "rgb(var(--chart-line-2))",
  );

  return (
    <ChartModal
      onClose={onClose}
      chartTitle="Metrics"
      entityName={tableName}
      dataSources={[readsSource, writesSource]}
      labels={["Reads", "Writes"]}
    />
  );
}
