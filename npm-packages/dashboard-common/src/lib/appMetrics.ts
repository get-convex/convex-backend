import { format } from "date-fns";
import useSWR from "swr";
import { FunctionExecution } from "system-udfs/convex/_system/frontend/common";
import { deploymentFetch } from "./fetching";
import {
  useDeploymentIsDisconnected,
  useDeploymentUrl,
  useDeploymentAuthHeader,
} from "./deploymentApi";
import { functionIdentifierValue } from "./functions/generateFileTree";

type TimeseriesResponse = [SerializedDate, number | null][];

function parseDate(date: SerializedDate): Date {
  let unixTsMs = date.secs_since_epoch * 1000;
  unixTsMs += date.nanos_since_epoch / 1_000_000;
  return new Date(unixTsMs);
}

const responseToTimeseries = (resp: TimeseriesResponse) =>
  resp.map(([time, metricOut]) => ({
    time: parseDate(time),
    metric: metricOut,
  }));

interface SerializedDate {
  secs_since_epoch: number;
  nanos_since_epoch: number;
}

function serializeDate(date: Date): SerializedDate {
  const unixTsSeconds = date.getTime() / 1000;
  const secsSinceEpoch = Math.floor(unixTsSeconds);
  const nanosSinceEpoch = Math.floor((unixTsSeconds - secsSinceEpoch) * 1e9);
  return {
    secs_since_epoch: secsSinceEpoch,
    nanos_since_epoch: nanosSinceEpoch,
  };
}

function appMetricsUrl(deploymentUrl: string): string {
  return `${deploymentUrl}/api/app_metrics`;
}

export type RequestFilter = {
  sessionId: string;
  clientRequestCounter: number;
};

export async function streamFunctionLogs(
  deploymentUrl: string,
  authHeader: string,
  cursorMs: number,
  requestFilter: RequestFilter | "all",
  signal: AbortSignal,
): Promise<{ entries: FunctionExecution[]; newCursor: number }> {
  const searchParams = new URLSearchParams({
    cursor: cursorMs.toString(),
  });
  if (requestFilter !== "all") {
    searchParams.set(
      "clientRequestCounter",
      requestFilter.clientRequestCounter.toString(),
    );
    searchParams.set("sessionId", requestFilter.sessionId);
  }
  const url = `${appMetricsUrl(
    deploymentUrl,
  )}/stream_function_logs?${searchParams.toString()}`;
  const response = await fetch(url, {
    headers: { Authorization: authHeader, "Convex-Client": "dashboard-0.0.0" },
    signal,
  });
  if (!response.ok) {
    throw new Error(await response.text());
  }
  return response.json();
}

export function useSchedulerLag() {
  const url = "/api/app_metrics/scheduled_job_lag";
  const isDisconnected = useDeploymentIsDisconnected();
  const deploymentUrl = useDeploymentUrl();
  const authHeader = useDeploymentAuthHeader();
  const fetcher = async () => {
    const start = new Date(Date.now() - 60 * 60 * 1000); // 1 hour ago
    const end = new Date();
    const windowArgs = {
      start: serializeDate(start),
      end: serializeDate(end),
      num_buckets: 60,
    };
    const window = JSON.stringify(windowArgs);
    const params = { window };
    const queryString = new URLSearchParams(params).toString();
    return deploymentFetch([
      deploymentUrl,
      `${url}?${queryString}`,
      authHeader,
    ]);
  };

  const { data: d } = useSWR(isDisconnected ? null : url, fetcher, {
    refreshInterval: 2.5 * 1000,
  });
  if (!d) {
    return undefined;
  }
  const buckets = responseToTimeseries(d as TimeseriesResponse);
  const data = buckets.map((value) =>
    value.metric
      ? {
          time: format(value.time, "h:mm a"),
          lag: Math.round(value.metric / 60),
        }
      : {
          time: format(value.time, "h:mm a"),
          lag: 0,
        },
  );
  return {
    data,
    xAxisKey: "time",
    lineKeys: [{ key: "lag", name: "Lag", color: "rgb(var(--brand-yellow))" }],
  };
}

type LatencyMetricsResponse = [number, TimeseriesResponse][];
type TopKMetricsResponse = [string, TimeseriesResponse][];

const multiResponseToTimeSeries = (
  resp: LatencyMetricsResponse | TopKMetricsResponse,
) => {
  const out = new Map();
  resp.forEach(([key, timeseries]) => {
    out.set(key, responseToTimeseries(timeseries));
  });
  return out;
};

export function useTopKCacheKey(
  kind: "cacheHitPercentage" | "failurePercentage",
) {
  const deploymentUrl = useDeploymentUrl();
  const route =
    kind === "cacheHitPercentage"
      ? "cache_hit_percentage_top_k"
      : "failure_percentage_top_k";

  return `${deploymentUrl}/api/app_metrics/${route}`;
}

export function useTopKFunctionMetrics(
  kind: "cacheHitPercentage" | "failurePercentage",
) {
  const url = `/api/app_metrics/${kind === "cacheHitPercentage" ? "cache_hit_percentage_top_k" : "failure_percentage_top_k"}`;
  const cacheKey = useTopKCacheKey(kind);
  const isDisconnected = useDeploymentIsDisconnected();
  const deploymentUrl = useDeploymentUrl();
  const authHeader = useDeploymentAuthHeader();
  const fetcher = async () => {
    const start = new Date(Date.now() - 60 * 60 * 1000); // 1 hour ago
    const end = new Date();
    const windowArgs = {
      start: serializeDate(start),
      end: serializeDate(end),
      num_buckets: 60,
    };
    const window = JSON.stringify(windowArgs);
    const params = { window, k: (3).toString() };
    const queryString = new URLSearchParams(params).toString();
    return deploymentFetch([
      deploymentUrl,
      `${url}?${queryString}`,
      authHeader,
    ]);
  };

  const { data: d } = useSWR(
    // The key here is not used by the fetcher, but
    // it's used by the SWR cache to differentiate between different calls.
    isDisconnected ? null : cacheKey,
    fetcher,
    {
      refreshInterval: 2.5 * 1000,
    },
  );
  if (!d) {
    return undefined;
  }
  const mapFunctionToBuckets = multiResponseToTimeSeries(
    d as TopKMetricsResponse,
  );
  const data = [];
  const lineKeys = [];
  const functions: string[] = [...mapFunctionToBuckets.keys()];
  const xAxisKey = "time";
  if (!mapFunctionToBuckets || !functions.length) {
    return null;
  }
  let hadDataAt = -1;
  for (const [i, bucket] of mapFunctionToBuckets.get(functions[0])!.entries()) {
    const dataPoint: any = {};
    dataPoint[xAxisKey] = format(bucket.time, "h:mm a");
    for (const f of functions) {
      const { metric } = mapFunctionToBuckets.get(f)![i];
      if (hadDataAt === -1) {
        hadDataAt = metric !== null ? i : hadDataAt;
      }
      dataPoint[identifierForMetricName(f)] =
        typeof metric === "number"
          ? metric
          : hadDataAt > -1
            ? kind === "cacheHitPercentage"
              ? 100
              : 0
            : null;
    }
    data.push(dataPoint);
  }

  const colorForFunction = new Map<string, string>();
  for (const f of functions) {
    if (f === "_rest") {
      colorForFunction.set(f, restColor);
      continue;
    }

    const colorIndex =
      [...f].reduce((acc, char) => acc + char.charCodeAt(0), 0) %
      lineColors.length;
    let color = lineColors[colorIndex];
    let attempts = 0;
    while (
      [...colorForFunction.values()].includes(color) &&
      attempts < lineColors.length
    ) {
      attempts++;
      color = lineColors[(colorIndex + attempts) % lineColors.length];
    }
    colorForFunction.set(f, color);
  }

  for (const [_, f] of functions.entries()) {
    const key = identifierForMetricName(f);
    const lineKey = {
      key,
      name: key,
      color: colorForFunction.get(f)!,
    };
    lineKeys.push(lineKey);
  }
  return {
    // If there's missing data, only show up to where we had data.
    // If there's only one data point, show 2 data points so the graph doesn't look strange.
    data: hadDataAt > -1 ? data.slice(hadDataAt === 59 ? 58 : hadDataAt) : data,
    xAxisKey,
    lineKeys,
  };
}

const restColor = "rgb(var(--chart-line-1))";
const lineColors = [
  "rgb(var(--chart-line-2))",
  "rgb(var(--chart-line-3))",
  "rgb(var(--chart-line-4))",
  "rgb(var(--chart-line-5))",
  "rgb(var(--chart-line-6))",
  "rgb(var(--chart-line-7))",
];

function identifierForMetricName(metric: string) {
  return metric === "_rest" ? metric : functionIdentifierValue(metric);
}
