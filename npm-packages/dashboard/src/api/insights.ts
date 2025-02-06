import { useCurrentDeployment } from "api/deployments";
import { useCurrentTeam } from "api/teams";

import { rootComponentPath, useUsageQuery } from "hooks/usageMetrics";
import omit from "lodash/omit";
import { useMemo } from "react";
import { useRouter } from "next/router";
import {
  itemIdentifier,
  useModuleFunctions,
} from "dashboard-common/lib/functions/FunctionsProvider";
import { functionIdentifierValue } from "dashboard-common/lib/functions/generateFileTree";

const queryIds = {
  bytesRead: "5bebdf6d-921a-42dc-9ee6-2d5c577111b6",
  bytesReadLowThreshold: "9dd6a290-4d03-4845-a219-24fbf3ff592d",
  bytesReadAvgByHour: "acae5783-62bf-44a4-a815-512a89980e34",
  bytesReadCountByHour: "f175ea1e-36a5-410e-b4c6-8229c4b079be",
  bytesReadCountByHourLowThreshold: "99c288f4-73ac-4631-9b01-476e18bb351d",
  bytesReadEvents: "87b175d7-4e44-432d-ab36-0fd03a2d3a2e",
  bytesReadEventsLowThreshold: "8ab70da7-84d3-496b-8b28-5cc0b72a30df",
  docsRead: "1bb76d3f-1e6d-46fa-8db9-ae8cb3cf9d75",
  docsReadLowThreshold: "86b808e0-f91d-48b4-bf6b-116b32c6f07f",
  docsReadAvgByHour: "d648397e-2af1-44ed-bb9f-8fea4b893657",
  docsReadCountByHour: "09a0c79f-ba7e-47f6-8068-e8c1690017a7",
  docsReadCountByHourLowThreshold: "d1bb2342-e012-446b-9bdf-c8092c2bbf3d",
  docsReadEvents: "59d1629f-c962-438a-bd67-a61e01acb2c6",
  docsReadEventsLowThreshold: "dd702e63-c942-4427-8df5-c1e297f7749f",
  occs: "985fea4b-c43c-41a8-a106-8f88cd8cdb9a",
  occsByHour: "82b42c49-4711-479e-b24c-94316e1d7616",
  occFailedEvents: "ab532018-7c86-40ca-a80a-e77122ff2968",
  occRetriedEvents: "1f27d560-5125-4350-87a2-0a2a478e9697",
};

export type InsightsSummaryData = {
  functionId: string;
  componentPath: string | null;
} & (
  | {
      kind: "bytesReadAverageThreshold";
      aboveThresholdCalls: number;
      totalCalls: number;
      avgBytesRead: number;
    }
  | {
      kind: "bytesReadCountThreshold";
      aboveThresholdCalls: number;
      totalCalls: number;
    }
  | {
      kind: "docsReadAverageThreshold";
      aboveThresholdCalls: number;
      totalCalls: number;
      avgDocsRead: number;
    }
  | {
      kind: "docsReadCountThreshold";
      aboveThresholdCalls: number;
      totalCalls: number;
    }
  | {
      kind: "occFailedPermanently";
      occCalls: number;
      totalCalls: number;
      occTableName: string;
    }
  | {
      kind: "occRetried";
      occCalls: number;
      totalCalls: number;
      occTableName: string;
    }
);

export function useInsightsPeriod() {
  // TODO: Allow configuring the period and surface the time range for insights
  return {
    from: new Date(new Date().setDate(new Date().getDate() - 2))
      .toISOString()
      .split("T")[0],
    to: new Date(new Date().setDate(new Date().getDate() + 1))
      .toISOString()
      .split("T")[0],
  };
}

export function useInsightsSummary(): InsightsSummaryData[] | undefined {
  const moduleFunctions = useModuleFunctions();

  const period = useInsightsPeriod();

  const team = useCurrentTeam();
  const deployment = useCurrentDeployment();
  const common = {
    deploymentName: deployment?.name,
    period,
    teamId: team?.id!,
    projectId: null,
    componentPrefix: null,
  };

  const { query } = useRouter();

  const { data: bytesReadThresholdData } = useUsageQuery({
    queryId: query.lowInsightsThreshold
      ? queryIds.bytesReadLowThreshold
      : queryIds.bytesRead,
    ...common,
  });
  const { data: docsReadThresholdData } = useUsageQuery({
    queryId: query.lowInsightsThreshold
      ? queryIds.docsReadLowThreshold
      : queryIds.docsRead,
    ...common,
  });
  const { data: occData } = useUsageQuery({
    queryId: queryIds.occs,
    ...common,
  });
  if (!bytesReadThresholdData || !docsReadThresholdData || !occData) {
    return undefined;
  }

  const mappedBytesReadThresholdData = bytesReadThresholdData.map((d) => ({
    functionId: d[0],
    componentPath: d[1] === rootComponentPath ? null : d[1],
    aboveThresholdCalls: Number(d[2]),
    totalCalls: Number(d[3]),
    avgBytesRead: Number(d[4]),
  }));

  const bytesReadAverageThresholdInsights: (InsightsSummaryData & {
    kind: "bytesReadAverageThreshold";
  })[] = mappedBytesReadThresholdData
    // Any function that has more than 50% of its calls above the threshold
    // is considered to be an average threshold violation and will be rendered
    // as an average
    .filter((d) => d.aboveThresholdCalls / d.totalCalls > 0.5)
    .map((d) => ({
      kind: "bytesReadAverageThreshold",
      functionId: d.functionId,
      componentPath: d.componentPath,
      aboveThresholdCalls: d.aboveThresholdCalls,
      totalCalls: d.totalCalls,
      avgBytesRead: d.avgBytesRead,
    }));

  const bytesReadCountInsights: (InsightsSummaryData & {
    kind: "bytesReadCountThreshold";
  })[] = mappedBytesReadThresholdData
    // Any function that has less than 50% of its calls above the threshold
    // is considered to be a count threshold violation and will be rendered
    // as a count. These are lower priority than the average threshold violations
    .filter((d) => d.aboveThresholdCalls / d.totalCalls <= 0.5)
    .map((d) => ({
      kind: "bytesReadCountThreshold",
      functionId: d.functionId,
      componentPath: d.componentPath,
      aboveThresholdCalls: d.aboveThresholdCalls,
      totalCalls: d.totalCalls,
    }));

  const mappedDocsReadThresholdData = docsReadThresholdData?.map((d) => ({
    functionId: d[0],
    componentPath: d[1] === rootComponentPath ? null : d[1],
    aboveThresholdCalls: Number(d[2]),
    totalCalls: Number(d[3]),
    avgDocsRead: Number(d[4]),
  }));

  const docsReadAverageThresholdInsights:
    | undefined
    | (InsightsSummaryData & {
        kind: "docsReadAverageThreshold";
      })[] = mappedDocsReadThresholdData
    // Any function that has more than 50% of its calls above the threshold
    // is considered to be an average threshold violation and will be rendered
    // as an average
    ?.filter((d) => d.aboveThresholdCalls / d.totalCalls > 0.5)
    .map((d) => ({
      kind: "docsReadAverageThreshold",
      functionId: d.functionId,
      componentPath: d.componentPath,
      aboveThresholdCalls: d.aboveThresholdCalls,
      totalCalls: d.totalCalls,
      avgDocsRead: d.avgDocsRead,
    }));

  const docsReadCountInsights:
    | undefined
    | (InsightsSummaryData & {
        kind: "docsReadCountThreshold";
      })[] = mappedDocsReadThresholdData
    // Any function that has less than 50% of its calls above the threshold
    // is considered to be a count threshold violation and will be rendered
    // as a count. These are lower priority than the average threshold violations
    ?.filter((d) => d.aboveThresholdCalls / d.totalCalls <= 0.5)
    .map((d) => ({
      kind: "docsReadCountThreshold",
      functionId: d.functionId,
      componentPath: d.componentPath,
      aboveThresholdCalls: d.aboveThresholdCalls,
      totalCalls: d.totalCalls,
    }));

  const mappedOccData = occData.map((d) => ({
    functionId: d[0],
    componentPath: d[1] === rootComponentPath ? null : d[2],
    occCalls: Number(d[2]),
    totalCalls: Number(d[3]),
    occTableName: d[4],
    permanentFailure: d[5] === "true",
  }));

  const occFailedPermanentlyInsights: (InsightsSummaryData & {
    kind: "occFailedPermanently";
  })[] = mappedOccData
    .filter((d) => d.permanentFailure)
    .map((d) => ({
      kind: "occFailedPermanently",
      ...omit(d, ["permanentFailure"]),
    }));

  const occRetriedInsights: (InsightsSummaryData & {
    kind: "occRetried";
  })[] = mappedOccData
    .filter((d) => !d.permanentFailure)
    .map((d) => ({
      kind: "occRetried",
      ...omit(d, ["permanentFailure"]),
    }))
    // Don't include retried insights that are also in the failed permanently insights.
    .filter((d): d is InsightsSummaryData & { kind: "occRetried" } =>
      occFailedPermanentlyInsights.some(
        (f) =>
          f.functionId === d.functionId &&
          f.componentPath === d.componentPath &&
          f.occTableName === d.occTableName,
      ),
    );

  const data: InsightsSummaryData[] = [
    ...bytesReadAverageThresholdInsights,
    ...(docsReadAverageThresholdInsights || []),
    ...occFailedPermanentlyInsights,
    ...bytesReadCountInsights,
    ...(docsReadCountInsights || []),
    ...occRetriedInsights,
  ];

  return data
    ? data
        // Filter out functions that don't exist
        .filter((insight) => {
          const id = functionIdentifierValue(
            insight.functionId,
            insight.componentPath ?? undefined,
          );
          return moduleFunctions.some((mf) => itemIdentifier(mf) === id);
        })
    : undefined;
}

type QueryIdType =
  | "occsByHour"
  | "bytesReadAvgByHour"
  | "bytesReadCountByHour"
  | "bytesReadCountByHourLowThreshold"
  | "docsReadAvgByHour"
  | "docsReadCountByHour"
  | "docsReadCountByHourLowThreshold";

function useDataByHour<T>({
  queryId,
  mapData,
  filterData,
  fillData,
}: {
  queryId: QueryIdType;
  mapData: (d: string[]) => T;
  filterData: (d: T) => boolean;
  fillData: (dateHour: string) => T;
}): T[] | undefined {
  const team = useCurrentTeam();
  const deployment = useCurrentDeployment();
  const period = useInsightsPeriod();
  const common = {
    deploymentName: deployment?.name,
    period,
    teamId: team?.id!,
    projectId: null,
    componentPrefix: null,
  };

  const { data } = useUsageQuery({
    queryId: queryIds[queryId],
    ...common,
  });

  if (!data) {
    return undefined;
  }

  const mapped = data.map(mapData);
  const filtered = mapped.filter(filterData);

  const filled = [];

  // For the hardcoded 3 day period, we expect up to 72 data points.
  // Fill in any missing data points with 0s.
  for (let i = 0; i < 72; i++) {
    const date = new Date(period.from);
    date.setHours(date.getHours() + i);
    if (date > new Date(new Date().setHours(new Date().getHours()))) {
      break;
    }
    const dateHour = date.toISOString().replace("T", " ").split(".")[0];
    const existing = filtered.find((d) => (d as any).dateHour === dateHour);
    if (existing) {
      filled.push(existing);
    } else {
      filled.push(fillData(dateHour));
    }
  }

  return filled;
}

export type OCCByHourData = {
  functionId: string;
  componentPath: string | null;
  dateHour: string;
  occCalls: number;
  occTableName: string;
  permanentFailure: boolean;
};

export function useOCCByHour({
  functionId,
  componentPath,
  tableName,
  permanentFailure,
}: {
  functionId: string;
  componentPath: string | null;
  tableName: string;
  permanentFailure: boolean;
}): OCCByHourData[] | undefined {
  const data = useDataByHour({
    queryId: "occsByHour",
    mapData: (d) => ({
      functionId: d[0],
      componentPath: d[1] === rootComponentPath ? null : d[1],
      dateHour: d[2],
      occCalls: Number(d[3]),
      occTableName: d[4],
      permanentFailure: d[5] === "true",
    }),
    filterData: (d) =>
      d.functionId === functionId &&
      d.componentPath === componentPath &&
      d.occTableName === tableName &&
      d.permanentFailure === permanentFailure,
    fillData: (dateHour) => ({
      functionId,
      componentPath,
      dateHour,
      occCalls: 0,
      occTableName: tableName,
      permanentFailure,
    }),
  });

  // For upsell sample data.
  const upsellData = useUpsellOCCData(functionId);
  if (upsellData) {
    return upsellData;
  }

  return data;
}

export type AverageByHourData = {
  functionId: string;
  componentPath: string | null;
  dateHour: string;
  avg: number;
};

export function useBytesReadAverageByHour({
  functionId,
  componentPath,
}: {
  functionId: string;
  componentPath: string | null;
}): AverageByHourData[] | undefined {
  const data = useDataByHour({
    queryId: "bytesReadAvgByHour",
    mapData: (d) => ({
      functionId: d[0],
      componentPath: d[1] === rootComponentPath ? null : d[1],
      dateHour: d[2],
      avg: Number(d[3]),
    }),
    filterData: (d) =>
      d.functionId === functionId && d.componentPath === componentPath,
    fillData: (dateHour) => ({
      functionId,
      componentPath,
      dateHour,
      avg: 0,
    }),
  });
  // For upsell sample data.
  const upsellData = useUpsellBytesReadAverageData(functionId);
  if (upsellData) {
    return upsellData;
  }

  return data;
}

export type CountByHourData = {
  functionId: string;
  componentPath: string | null;
  dateHour: string;
  count: number;
};

export function useBytesReadCountByHour({
  functionId,
  componentPath,
}: {
  functionId: string;
  componentPath: string | null;
}): CountByHourData[] | undefined {
  const { query } = useRouter();
  // For upsell sample data.
  const upsellData = useUpsellBytesReadCountData(functionId);
  const data = useDataByHour({
    queryId: query.lowInsightsThreshold
      ? "bytesReadCountByHourLowThreshold"
      : "bytesReadCountByHour",
    mapData: (d) => ({
      functionId: d[0],
      componentPath: d[1] === rootComponentPath ? null : d[1],
      dateHour: d[2],
      count: Number(d[3]),
    }),
    filterData: (d) =>
      d.functionId === functionId && d.componentPath === componentPath,
    fillData: (dateHour) => ({
      functionId,
      componentPath,
      dateHour,
      count: 0,
    }),
  });
  if (upsellData) {
    return upsellData;
  }
  return data;
}

export function useDocumentsReadAverageByHour({
  functionId,
  componentPath,
}: {
  functionId: string;
  componentPath: string | null;
}): AverageByHourData[] | undefined {
  const data = useDataByHour({
    queryId: "docsReadAvgByHour",
    mapData: (d) => ({
      functionId: d[0],
      componentPath: d[1] === rootComponentPath ? null : d[1],
      dateHour: d[2],
      avg: Number(d[3]),
    }),
    filterData: (d) =>
      d.functionId === functionId && d.componentPath === componentPath,
    fillData: (dateHour) => ({
      functionId,
      componentPath,
      dateHour,
      avg: 0,
    }),
  });

  return data;
}

export function useDocumentsReadCountByHour({
  functionId,
  componentPath,
}: {
  functionId: string;
  componentPath: string | null;
}): CountByHourData[] | undefined {
  const { query } = useRouter();
  const data = useDataByHour({
    queryId: query.lowInsightsThreshold
      ? "docsReadCountByHourLowThreshold"
      : "docsReadCountByHour",
    mapData: (d) => ({
      functionId: d[0],
      componentPath: d[1] === rootComponentPath ? null : d[1],
      dateHour: d[2],
      count: Number(d[3]),
    }),
    filterData: (d) =>
      d.functionId === functionId && d.componentPath === componentPath,
    fillData: (dateHour) => ({
      functionId,
      componentPath,
      dateHour,
      count: 0,
    }),
  });

  return data;
}

function useUpsellOCCData(functionId: string) {
  return useMemo(() => {
    if (
      UPSELL_INSIGHTS.some(
        (i) =>
          (i.kind === "occRetried" || i.kind === "occFailedPermanently") &&
          i.functionId === functionId,
      )
    ) {
      return Array.from({ length: 72 }, (_, i) => {
        const date = new Date();
        date.setHours(date.getHours() - 71 + i);
        const dateHour = date.toISOString().replace("T", " ").split(".")[0];
        return {
          functionId,
          componentPath: null,
          dateHour,
          occCalls: Math.floor(Math.random() * 1000) + 1,
          occTableName: "table1",
          permanentFailure: functionId === "mutations/_writeAllTheData",
        };
      });
    }
    return null;
  }, [functionId]);
}

function useUpsellBytesReadAverageData(functionId: string) {
  return useMemo(() => {
    if (
      UPSELL_INSIGHTS.some(
        (i) =>
          i.kind === "bytesReadAverageThreshold" && i.functionId === functionId,
      )
    ) {
      return Array.from({ length: 72 }, (_, i) => {
        const date = new Date();
        date.setHours(date.getHours() - 71 + i);
        const dateHour = date.toISOString().replace("T", " ").split(".")[0];
        return {
          functionId,
          componentPath: null,
          dateHour,
          avg: i * 100 * 1024,
        };
      });
    }
  }, [functionId]);
}

function useUpsellBytesReadCountData(functionId: string) {
  return useMemo(() => {
    if (
      UPSELL_INSIGHTS.some(
        (i) =>
          i.kind === "bytesReadCountThreshold" && i.functionId === functionId,
      )
    ) {
      return Array.from({ length: 72 }, (_, i) => {
        const date = new Date();
        date.setHours(date.getHours() - 71 + i);
        const dateHour = date.toISOString().replace("T", " ").split(".")[0];
        return {
          functionId,
          componentPath: null,
          dateHour,
          count: Math.floor(Math.random() * 1000) + 1,
        };
      });
    }
    return null;
  }, [functionId]);
}

// Insights used for the upsells screen
export const UPSELL_INSIGHTS: InsightsSummaryData[] = [
  {
    kind: "bytesReadAverageThreshold",
    functionId: "queries/_unoptimizedFunction",
    componentPath: null,
    aboveThresholdCalls: 10,
    totalCalls: 20,
    avgBytesRead: 7 * 1024 * 1024,
  },
  {
    kind: "bytesReadAverageThreshold",
    functionId: "queries/_unoptimizedFunction2",
    componentPath: null,
    aboveThresholdCalls: 321,
    totalCalls: 1000,
    avgBytesRead: 7 * 1024 * 1024,
  },
  {
    kind: "occFailedPermanently",
    functionId: "mutations/_writeAllTheData",
    componentPath: null,
    occCalls: 5,
    totalCalls: 10,
    occTableName: "table1",
  },
  {
    kind: "bytesReadCountThreshold",
    functionId: "queries/_unoptimizedFunction3",
    componentPath: null,
    aboveThresholdCalls: 5,
    totalCalls: 10000,
  },
  {
    kind: "occRetried",
    functionId: "mutations/_writeSomeOfTheData",
    componentPath: null,
    occCalls: 5,
    totalCalls: 10,
    occTableName: "table2",
  },
];

export type ReadEventData = {
  timestamp: string;
  executionId: string;
  requestId: string;
  totalCount: number;
  status: string;
  events: { tableName: string; count: number }[];
};

export function useBytesReadEvents({
  functionId,
  componentPath,
}: {
  functionId: string;
  componentPath: string | null;
}): ReadEventData[] | undefined {
  const { query } = useRouter();
  const team = useCurrentTeam();
  const deployment = useCurrentDeployment();
  const period = useInsightsPeriod();
  const common = {
    deploymentName: deployment?.name,
    functionId,
    period,
    teamId: team?.id!,
    projectId: null,
    componentPrefix: componentPath,
  };
  const { data } = useUsageQuery({
    queryId: query.lowInsightsThreshold
      ? queryIds.bytesReadEventsLowThreshold
      : queryIds.bytesReadEvents,
    ...common,
  });
  if (!data) {
    return undefined;
  }
  return data.map((d: string[]) => ({
    timestamp: d[0],
    executionId: d[1],
    requestId: d[2],
    status: d[3],
    totalCount: Number(d[4]),
    events: JSON.parse(d[5]).map(([tableName, count]: [string, string]) => ({
      tableName,
      count: Number(count),
    })),
  }));
}

export function useDocumentsReadEvents({
  functionId,
  componentPath,
}: {
  functionId: string;
  componentPath: string | null;
}): ReadEventData[] | undefined {
  const { query } = useRouter();
  const team = useCurrentTeam();
  const deployment = useCurrentDeployment();
  const period = useInsightsPeriod();
  const common = {
    deploymentName: deployment?.name,
    functionId,
    period,
    teamId: team?.id!,
    projectId: null,
    componentPrefix: componentPath,
  };
  const { data } = useUsageQuery({
    queryId: query.lowInsightsThreshold
      ? queryIds.docsReadEventsLowThreshold
      : queryIds.docsReadEvents,
    ...common,
  });
  if (!data) {
    return undefined;
  }
  return data.map((d: string[]) => ({
    timestamp: d[0],
    executionId: d[1],
    requestId: d[2],
    status: d[3],
    totalCount: Number(d[4]),
    events: JSON.parse(d[5]).map(([tableName, count]: [string, string]) => ({
      tableName,
      count: Number(count),
    })),
  }));
}

export type OCCEventData = {
  timestamp: string;
  executionId: string;
  requestId: string;
  occDocumentId: string;
  occWriteSource: string | undefined;
};

export function useOCCFailedEvents({
  functionId,
  componentPath,
  tableName,
}: {
  functionId: string;
  componentPath: string | null;
  tableName: string;
}): OCCEventData[] | undefined {
  const team = useCurrentTeam();
  const deployment = useCurrentDeployment();
  const period = useInsightsPeriod();
  const common = {
    deploymentName: deployment?.name,
    functionId,
    tableName,
    period,
    teamId: team?.id!,
    projectId: null,
    componentPrefix: componentPath,
  };
  const { data } = useUsageQuery({
    queryId: queryIds.occFailedEvents,
    ...common,
  });
  if (!data) {
    return undefined;
  }
  return data.map((d: string[]) => ({
    timestamp: d[0],
    executionId: d[1],
    requestId: d[2],
    occDocumentId: d[3],
    occWriteSource: d[4],
  }));
}

export type OCCRetriedEventData = OCCEventData & {
  occRetryCount: number;
};

export function useOCCRetriedEvents({
  functionId,
  componentPath,
  tableName,
}: {
  functionId: string;
  componentPath: string | null;
  tableName: string;
}): OCCRetriedEventData[] | undefined {
  const team = useCurrentTeam();
  const deployment = useCurrentDeployment();
  const period = useInsightsPeriod();
  const common = {
    deploymentName: deployment?.name,
    functionId,
    tableName,
    period,
    teamId: team?.id!,
    projectId: null,
    componentPrefix: componentPath,
  };
  const { data } = useUsageQuery({
    queryId: queryIds.occRetriedEvents,
    ...common,
  });
  if (!data) {
    return undefined;
  }

  return data.map((d: string[]) => ({
    timestamp: d[0],
    executionId: d[1],
    requestId: d[2],
    occDocumentId: d[3],
    occRetryCount: Number(d[4]),
    occWriteSource: d[5],
  }));
}
