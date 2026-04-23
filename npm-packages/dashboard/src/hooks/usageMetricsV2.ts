import { DatabricksQueryId, DateRange, useUsageQuery } from "api/usage";

const QUERY_IDS_V2: {
  summary: DatabricksQueryId;
  functionBreakdown: DatabricksQueryId;
  deploymentsByClassAndRegion: DatabricksQueryId;
} = {
  summary: "b63fe48d-320c-401a-8682-0a0b36b50e2b",
  functionBreakdown: "90ec3ee0-720f-4e67-94a2-75ecd278b3c6",
  deploymentsByClassAndRegion: "dfc73057-1948-4b99-a3bf-9ae802a395ee",
};

const BY_PROJECT_QUERY_IDS_V2: {
  databaseStorageByProjectAndClass: DatabricksQueryId;
  databaseStorageByTable: DatabricksQueryId;
  documentCountByTable: DatabricksQueryId;
  databaseIOByProjectAndClass: DatabricksQueryId;
  functionCallsByProjectAndClass: DatabricksQueryId;
  storageCallsByProjectAndClass: DatabricksQueryId;
  computeByProject: DatabricksQueryId;
  computeByProjectSelfServe: DatabricksQueryId;
  fileStorageByProject: DatabricksQueryId;
  searchStorageByProject: DatabricksQueryId;
  dataEgressByProject: DatabricksQueryId;
  searchQueriesByProject: DatabricksQueryId;
} = {
  databaseStorageByProjectAndClass: "489b0f87-6b3a-4dfe-a327-f2965b5c2977",
  databaseStorageByTable: "017c5977-3002-40ca-96af-31868e70e611",
  documentCountByTable: "28646a64-f234-44c2-b763-ecb63d43ad24",
  databaseIOByProjectAndClass: "9f606f77-521d-44bb-83ef-b1057b0fb1c9",
  functionCallsByProjectAndClass: "77a4e5bd-aa82-43e7-85a4-89897cecaa05",
  storageCallsByProjectAndClass: "90c9d3b3-d93e-4583-a054-dbb2f9dad5a3",
  computeByProject: "45921934-b4a2-4c91-9ba2-8987fba6e8e3",
  computeByProjectSelfServe: "038e5492-6de5-4ddb-86b4-761e19b4d2ab",
  fileStorageByProject: "72add9df-4ef2-47fe-9942-194dfbb72088",
  searchStorageByProject: "87f2b0b2-024c-4c2a-bf81-8a3c0cab1b82",
  dataEgressByProject: "67ce838f-b2d0-4cda-9a2e-580c6d134466",
  searchQueriesByProject: "48ae8bb1-ec17-41db-9e35-7c774296c5ac",
};

// --- Types ---

export type UsageSummaryRowV2 = {
  deploymentClass: string;
  region: string;
  databaseStorage: number;
  databaseIO: number;
  functionCalls: number;
  queryMutationCompute: number; // GB-hours
  actionComputeConvex: number; // GB-hours
  actionComputeNode: number; // GB-hours
  fileStorage: number;
  searchStorage: number;
  dataEgress: number;
  searchQueries: number;
  actionComputeUser: number; // GB-hours — corrected non-node compute for business plans
};

export interface AggregatedFunctionMetricsV2 {
  function: string;
  projectId: number;
  callCount: number;
  databaseIngressSize: number;
  databaseEgressSize: number;
  textSearchGb: number; // query-GB
  vectorSearchGb: number; // query-GB
  queryMutationComputeTime: number; // GB-hours
  actionComputeConvexTime: number; // GB-hours
  actionComputeNodeTime: number; // GB-hours
  dataEgress: number; // bytes
  deploymentName?: string;
  componentPath: string;
}

export interface DailyPerTagMetricsByProjectAndClass {
  ds: string;
  projectId: number | "_rest";
  deploymentClass: string;
  metrics: { tag: string; value: number }[];
}

export interface DailyMetricByTable {
  ds: string;
  projectId: number | "_rest";
  tableName: string;
  value: number;
}

export interface DailyMetricByProject {
  ds: string;
  projectId: number | "_rest";
  value: number;
}

export interface DailyPerTagMetricsByProject {
  ds: string;
  projectId: number | "_rest";
  metrics: { tag: string; value: number }[];
}

export interface DailyDeploymentsByClassAndRegion {
  ds: string;
  deploymentClass: string;
  region: string;
  count: number;
}

// --- Helpers ---

function parseProjectId(projectId: string | number): number | "_rest" {
  if (projectId === "_rest") {
    return "_rest";
  }
  return Number(projectId);
}

// --- Hooks ---

export function useUsageTeamSummaryV2(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
) {
  const { data, error } = useUsageQuery({
    queryId: QUERY_IDS_V2.summary,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  if (!data) {
    return { data: undefined, error: undefined };
  }

  return {
    data: data.map(
      ([
        _teamId,
        deploymentClass,
        region,
        databaseStorage,
        databaseIO,
        functionCalls,
        queryMutationCompute,
        actionComputeConvex,
        actionComputeNode,
        fileStorage,
        searchStorage,
        dataEgress,
        searchQueries,
        actionComputeUser,
      ]) =>
        ({
          deploymentClass,
          region,
          databaseStorage: Number(databaseStorage),
          databaseIO: Number(databaseIO),
          functionCalls: Number(functionCalls),
          queryMutationCompute: Number(queryMutationCompute) / 60 / 60,
          actionComputeConvex: Number(actionComputeConvex) / 60 / 60,
          actionComputeNode: Number(actionComputeNode) / 60 / 60,
          fileStorage: Number(fileStorage),
          searchStorage: Number(searchStorage),
          dataEgress: Number(dataEgress),
          searchQueries: Number(searchQueries),
          actionComputeUser: Number(actionComputeUser) / 60 / 60,
        }) satisfies UsageSummaryRowV2,
    ),
    error: undefined,
  };
}

export function useUsageTeamMetricsByFunctionV2(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: AggregatedFunctionMetricsV2[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: QUERY_IDS_V2.functionBreakdown,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
      ([
        _teamId,
        functionName,
        projectIdField,
        callCount,
        databaseIngressSize,
        databaseEgressSize,
        textSearchGb,
        vectorSearchGb,
        queryMutationComputeTime,
        actionComputeConvexTime,
        actionComputeNodeTime,
        dataEgress,
        deploymentName,
        componentPath,
      ]) => ({
        function: functionName,
        projectId: Number(projectIdField),
        callCount: Number(callCount),
        databaseIngressSize: Number(databaseIngressSize),
        databaseEgressSize: Number(databaseEgressSize),
        textSearchGb: Number(textSearchGb),
        vectorSearchGb: Number(vectorSearchGb),
        queryMutationComputeTime: Number(queryMutationComputeTime) / 60 / 60,
        actionComputeConvexTime: Number(actionComputeConvexTime) / 60 / 60,
        actionComputeNodeTime: Number(actionComputeNodeTime) / 60 / 60,
        dataEgress: Number(dataEgress),
        deploymentName,
        componentPath,
      }),
    ),
    error: undefined,
  };
}

// Daily by-project hooks (with deployment class)

export function useDatabaseStoragePerDayByProjectAndClassV2(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetricsByProjectAndClass[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: BY_PROJECT_QUERY_IDS_V2.databaseStorageByProjectAndClass,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
      ([
        _teamId,
        projectId,
        deploymentClass,
        ds,
        documentStorage,
        indexStorage,
      ]) => ({
        ds,
        projectId: parseProjectId(projectId),
        deploymentClass,
        metrics: [
          { tag: "document", value: Number(documentStorage) },
          { tag: "index", value: Number(indexStorage) },
        ],
      }),
    ),
    error: undefined,
  };
}

export function useDatabaseStoragePerDayByTableV2(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: DailyMetricByTable[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: BY_PROJECT_QUERY_IDS_V2.databaseStorageByTable,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
      ([
        _teamId,
        rowProjectId,
        tableName,
        ds,
        documentStorage,
        indexStorage,
      ]) => ({
        ds,
        projectId: parseProjectId(rowProjectId),
        tableName,
        value: Number(documentStorage) + Number(indexStorage),
      }),
    ),
    error: undefined,
  };
}

export function useDocumentCountPerDayByTableV2(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: DailyMetricByTable[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: BY_PROJECT_QUERY_IDS_V2.documentCountByTable,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
      ([_teamId, rowProjectId, tableName, ds, documentCount]) => ({
        ds,
        projectId: parseProjectId(rowProjectId),
        tableName,
        value: Number(documentCount),
      }),
    ),
    error: undefined,
  };
}

export function useDatabaseIOPerDayByProjectAndClassV2(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetricsByProjectAndClass[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: BY_PROJECT_QUERY_IDS_V2.databaseIOByProjectAndClass,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
      ([_teamId, projectId, deploymentClass, ds, ingressSize, egressSize]) => ({
        ds,
        projectId: parseProjectId(projectId),
        deploymentClass,
        metrics: [
          { tag: "egress", value: Number(egressSize) },
          { tag: "ingress", value: Number(ingressSize) },
        ],
      }),
    ),
    error: undefined,
  };
}

export function useComputePerDayByProjectV2(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetricsByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: BY_PROJECT_QUERY_IDS_V2.computeByProject,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
      ([
        _teamId,
        projectId,
        ds,
        queryMutationGbS,
        actionConvexGbS,
        actionNodeGbS,
      ]) => ({
        ds,
        projectId: parseProjectId(projectId),
        metrics: [
          {
            tag: "queryMutation",
            value: Number(queryMutationGbS) / 60 / 60,
          },
          {
            tag: "actionConvex",
            value: Number(actionConvexGbS) / 60 / 60,
          },
          { tag: "actionNode", value: Number(actionNodeGbS) / 60 / 60 },
        ],
      }),
    ),
    error: undefined,
  };
}

export function useComputePerDayByProjectSelfServeV2(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetricsByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: BY_PROJECT_QUERY_IDS_V2.computeByProjectSelfServe,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
      ([_teamId, projectId, ds, actionConvexGbS, actionNodeGbS]) => ({
        ds,
        projectId: parseProjectId(projectId),
        metrics: [
          {
            tag: "actionConvex",
            value: Number(actionConvexGbS) / 60 / 60,
          },
          { tag: "actionNode", value: Number(actionNodeGbS) / 60 / 60 },
        ],
      }),
    ),
    error: undefined,
  };
}

export function useFunctionCallsPerDayByProjectAndClassV2(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetricsByProjectAndClass[] | undefined; error: any } {
  const { data: functionData, error: functionError } = useUsageQuery({
    queryId: BY_PROJECT_QUERY_IDS_V2.functionCallsByProjectAndClass,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  const { data: storageData, error: storageError } = useUsageQuery({
    queryId: BY_PROJECT_QUERY_IDS_V2.storageCallsByProjectAndClass,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (functionError || storageError) {
    return { data: undefined, error: functionError || storageError };
  }

  const metrics = functionData?.map(
    ([
      _teamId,
      projectId,
      deploymentClass,
      ds,
      cachedQueries,
      uncachedQueries,
      mutations,
      actions,
      httpActions,
    ]) => ({
      ds,
      projectId: parseProjectId(projectId),
      deploymentClass,
      metrics: [
        { tag: "uncached_query", value: Number(uncachedQueries) },
        { tag: "cached_query", value: Number(cachedQueries) },
        { tag: "mutation", value: Number(mutations) },
        { tag: "action", value: Number(actions) },
        { tag: "http_action", value: Number(httpActions) },
      ],
    }),
  );

  // Augment with storage calls data
  const storageDataByKey = (storageData || []).reduce(
    (acc, [_teamId, projectId, deploymentClass, ds, storageCalls]) => {
      const key = `${ds}-${projectId}-${deploymentClass}`;
      acc[key] = Number(storageCalls);
      return acc;
    },
    {} as Record<string, number>,
  );
  for (const metric of metrics || []) {
    const key = `${metric.ds}-${metric.projectId}-${metric.deploymentClass}`;
    const storageCalls = storageDataByKey[key];
    if (storageCalls) {
      metric.metrics.push({ tag: "storage_api", value: storageCalls });
      delete storageDataByKey[key];
    }
  }

  return { data: metrics, error: undefined };
}

// Daily by-project hooks (without deployment class)

export function useFileStoragePerDayByProjectV2(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: DailyMetricByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: BY_PROJECT_QUERY_IDS_V2.fileStorageByProject,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(([_teamId, projectId, ds, fileStorage]) => ({
      ds,
      projectId: parseProjectId(projectId),
      value: Number(fileStorage),
    })),
    error: undefined,
  };
}

export function useSearchStoragePerDayByProjectV2(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetricsByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: BY_PROJECT_QUERY_IDS_V2.searchStorageByProject,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
      ([_teamId, projectId, ds, textSearchStorage, vectorStorage]) => ({
        ds,
        projectId: parseProjectId(projectId),
        metrics: [
          { tag: "textSearch", value: Number(textSearchStorage) },
          { tag: "vector", value: Number(vectorStorage) },
        ],
      }),
    ),
    error: undefined,
  };
}

export function useDataEgressPerDayByProjectV2(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetricsByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: BY_PROJECT_QUERY_IDS_V2.dataEgressByProject,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
      ([
        _teamId,
        projectId,
        ds,
        servingEgress,
        userFunctionEgress,
        cloudRestore,
        cloudBackup,
        snapshotExport,
        snapshotImport,
        fetchEgress,
        logStreamEgress,
        streamingExportEgress,
      ]) => ({
        ds,
        projectId: parseProjectId(projectId),
        metrics: [
          { tag: "fetchEgress", value: Number(fetchEgress) },
          { tag: "logStream", value: Number(logStreamEgress) },
          { tag: "streamingExport", value: Number(streamingExportEgress) },
          { tag: "servingEgress", value: Number(servingEgress) },
          { tag: "userFunctionEgress", value: Number(userFunctionEgress) },
          {
            tag: "backup",
            value: Number(cloudBackup) + Number(snapshotExport),
          },
          {
            tag: "restore",
            value: Number(cloudRestore) + Number(snapshotImport),
          },
        ],
      }),
    ),
    error: undefined,
  };
}

export function useSearchQueriesPerDayByProjectV2(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetricsByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: BY_PROJECT_QUERY_IDS_V2.searchQueriesByProject,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
      ([_teamId, projectId, ds, textSearchGb, vectorSearchGb]) => ({
        ds,
        projectId: parseProjectId(projectId),
        metrics: [
          { tag: "textSearch", value: Number(textSearchGb) },
          { tag: "vectorSearch", value: Number(vectorSearchGb) },
        ],
      }),
    ),
    error: undefined,
  };
}

// Deployments

export function useDeploymentsByClassAndRegionV2(
  teamId: number,
  period: DateRange | null,
): { data: DailyDeploymentsByClassAndRegion[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: QUERY_IDS_V2.deploymentsByClassAndRegion,
    teamId,
    projectId: null,
    period,
    componentPrefix: null,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(([_teamId, ds, deploymentClass, region, count]) => ({
      ds,
      deploymentClass,
      region,
      count: Number(count),
    })),
    error: undefined,
  };
}
