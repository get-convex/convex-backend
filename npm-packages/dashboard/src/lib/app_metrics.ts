import { UdfType } from "system-udfs/convex/_system/frontend/common";

export type UdfMetric = "invocations" | "errors" | "cacheHits" | "cacheMisses";
export type TableMetric = "rowsRead" | "rowsWritten";

export type Bucket = {
  // Start time for the bucket
  time: Date;
  metric: number | null;
};
export type Timeseries = Bucket[];

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

function parseDate(date: SerializedDate): Date {
  let unixTsMs = date.secs_since_epoch * 1000;
  unixTsMs += date.nanos_since_epoch / 1_000_000;
  return new Date(unixTsMs);
}

function appMetricsUrl(deploymentUrl: string): string {
  return `${deploymentUrl}/api/app_metrics`;
}

type TimeseriesResponse = [SerializedDate, number | null][];

const responseToTimeseries = (resp: TimeseriesResponse) =>
  resp.map(([time, metricOut]) => ({
    time: parseDate(time),
    metric: metricOut,
  }));

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

/**
 * Query a UDF rate (see the `UdfMetric` type) for the time window bounded by `start` and `end`. You can
 * control the number of samples returned (and, therefore, the metric's resolution) with `numBuckets`.
 */
export async function udfRate(args: {
  deploymentUrl: string;
  udfIdentifier: string;
  componentPath?: string;
  udfType: UdfType;
  start: Date;
  metric: UdfMetric;
  end: Date;
  numBuckets: number;
  authHeader: string;
}): Promise<Timeseries> {
  const searchParams = new URLSearchParams();
  searchParams.set("metric", args.metric);
  const responseJson = await getUdfMetric({
    ...args,
    searchParams,
    endpoint: "udf_rate",
  });
  return responseToTimeseries(responseJson);
}

/**
 * Query the cache hit rate percentage (as a number between 0% and 100%) for a given time window. Note that
 * the returned timeseries may have `null` for a bucket if there were no requests in that time interval.
 */
export async function cacheHitPercentage(args: {
  deploymentUrl: string;
  udfIdentifier: string;
  componentPath?: string;
  udfType: UdfType;
  start: Date;
  end: Date;
  numBuckets: number;
  authHeader: string;
}): Promise<Timeseries> {
  const responseJson = await getUdfMetric({
    ...args,
    searchParams: new URLSearchParams(),
    endpoint: "cache_hit_percentage",
  });
  return responseToTimeseries(responseJson);
}

/**
 * Query multiple percentiles (specified as a number between 0 and 100, inclusive) for UDF execution latency.
 */
export async function latencyPercentiles(args: {
  deploymentUrl: string;
  udfIdentifier: string;
  componentPath?: string;
  udfType: UdfType;
  percentiles: number[];
  start: Date;
  end: Date;
  numBuckets: number;
  authHeader: string;
}): Promise<Map<number, Timeseries>> {
  const searchParams = new URLSearchParams();
  searchParams.set("percentiles", JSON.stringify(args.percentiles));
  const responseJson = await getUdfMetric({
    ...args,
    searchParams,
    endpoint: "latency_percentiles",
  });
  return multiResponseToTimeSeries(responseJson);
}

async function getUdfMetric(args: {
  deploymentUrl: string;
  udfIdentifier: string;
  componentPath?: string;
  udfType: UdfType;
  searchParams: URLSearchParams;
  endpoint: string;
  start: Date;
  end: Date;
  numBuckets: number;
  authHeader: string;
}) {
  const { searchParams } = args;
  searchParams.set("path", args.udfIdentifier);
  if (args.componentPath) {
    searchParams.set("componentPath", args.componentPath);
  }
  const windowArgs = {
    start: serializeDate(args.start),
    end: serializeDate(args.end),
    num_buckets: args.numBuckets,
  };
  searchParams.set("window", JSON.stringify(windowArgs));
  searchParams.set("udfType", args.udfType);
  const url = `${appMetricsUrl(args.deploymentUrl)}/${
    args.endpoint
  }?${searchParams.toString()}`;
  const response = await fetch(url, {
    headers: {
      Authorization: args.authHeader,
      "Convex-Client": "dashboard-0.0.0",
    },
  });
  if (!response.ok) {
    throw new Error(await response.text());
  }
  return response.json();
}

/**
 * Query a Table rate (see the `TableMetric` type).
 */
export async function tableRate(
  deploymentUrl: string,
  tableName: string,
  metric: TableMetric,
  start: Date,
  end: Date,
  numBuckets: number,
  authHeader: string,
): Promise<Timeseries> {
  const windowArgs = {
    start: serializeDate(start),
    end: serializeDate(end),
    num_buckets: numBuckets,
  };
  const name = encodeURIComponent(tableName);
  const window = encodeURIComponent(JSON.stringify(windowArgs));
  const url = `${appMetricsUrl(
    deploymentUrl,
  )}/table_rate?name=${name}&metric=${metric}&window=${window}`;
  const response = await fetch(url, {
    headers: { Authorization: authHeader, "Convex-Client": "dashboard-0.0.0" },
  });
  if (!response.ok) {
    throw new Error(await response.text());
  }
  const respJSON: TimeseriesResponse = await response.json();
  return responseToTimeseries(respJSON);
}
