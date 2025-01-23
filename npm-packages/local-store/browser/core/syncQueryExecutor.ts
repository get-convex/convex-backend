import { anyApi } from "convex/server";
import {
  IndexRangeRequest,
  SyncQueryResult,
  ConvexSubscriptionId,
  TableName,
  IndexName,
  PageArguments,
} from "../../shared/types";
import { CoreResponse } from "./protocol";
import { LoadingError, LocalDbReaderImpl } from "../localDbReader";
import { GenericId, Value } from "convex/values";
import { CopyOnWriteLocalStore } from "./localStore";
import { LOG2_PAGE_SIZE, SingleIndexRangeExecutor } from "./paginator";
import { createQueryToken } from "../../shared/queryTokens";

export type SyncQuery = (ctx: { localDb: any }, args: any) => Value;

export type SyncQueryExecutionResult = {
  responses: CoreResponse[];
  pagesRead: ConvexSubscriptionId[];
  newPage: {
    subscriptionId: ConvexSubscriptionId;
    args: PageArguments;
  } | null;
  result: SyncQueryResult;
};

function getSyncFunction(tableName: TableName, indexName: IndexName) {
  return anyApi.sync[tableName][indexName];
}

export function executeSyncQuery(
  syncSchema: any,
  syncQueryFn: SyncQuery,
  syncQueryArgs: any,
  localStore: CopyOnWriteLocalStore,
): SyncQueryExecutionResult {
  let syncQueryResult: SyncQueryResult | undefined;
  let newPage: {
    subscriptionId: ConvexSubscriptionId;
    args: PageArguments;
  } | null = null;
  const responses: CoreResponse[] = [];
  const pagesRead: Set<ConvexSubscriptionId> = new Set();

  const handleRangeRequest = (rangeRequest: IndexRangeRequest) => {
    const singleIndexRangeExecutor = new SingleIndexRangeExecutor(
      rangeRequest,
      syncSchema,
      localStore,
    );

    const result = singleIndexRangeExecutor.tryFulfill();
    switch (result.state) {
      case "fulfilled":
        for (const pageSubscriptionId of result.pageSubscriptionIds) {
          pagesRead.add(pageSubscriptionId);
        }
        return result.results;
      case "waitingOnLoadingPage": {
        for (const pageSubscriptionId of result.loadingPageSubscriptionIds) {
          pagesRead.add(pageSubscriptionId);
        }
        syncQueryResult = { kind: "loading" };
        throw new LoadingError();
      }
      case "needsMorePages": {
        result.existingPageSubscriptionIds.forEach((id) => {
          pagesRead.add(id);
        });
        const pageArgs = {
          syncTableName: rangeRequest.tableName,
          index: rangeRequest.indexName,
          target: result.targetKey,
          log2PageSize: LOG2_PAGE_SIZE,
        };
        const pageSubscriptionId = createQueryToken(
          getSyncFunction(rangeRequest.tableName, rangeRequest.indexName),
          pageArgs,
        );
        newPage = { subscriptionId: pageSubscriptionId, args: pageArgs };
        pagesRead.add(pageSubscriptionId);
        responses.push({
          recipient: "Network",
          kind: "sendQueryToNetwork",
          syncFunction: getSyncFunction(
            rangeRequest.tableName,
            rangeRequest.indexName,
          ),
          pageRequest: pageArgs,
        });
        syncQueryResult = { kind: "loading" };
        throw new LoadingError();
      }
    }
  };

  const loadObject = (tableName: TableName, id: GenericId<any>) => {
    const result = localStore.loadObject(tableName, id);
    if (result === undefined) {
      syncQueryResult = { kind: "loading" };
      throw new LoadingError();
    }
    return result;
  };

  const localDb = new LocalDbReaderImpl(
    syncSchema,
    handleRangeRequest,
    loadObject,
  );
  try {
    const result = syncQueryFn({ localDb }, syncQueryArgs);
    if (result instanceof Promise) {
      syncQueryResult = {
        kind: "loaded",
        status: "error",
        error: new Error("Sync query returned a promise"),
      };
    }
    if (syncQueryResult === undefined) {
      syncQueryResult = { kind: "loaded", status: "success", value: result };
    }
  } catch (e) {
    if (e instanceof LoadingError) {
      syncQueryResult = { kind: "loading" };
    } else if (syncQueryResult === undefined) {
      syncQueryResult = {
        kind: "loaded",
        status: "error",
        error: e,
      };
    }
  }
  return {
    responses,
    pagesRead: Array.from(pagesRead),
    newPage,
    result: syncQueryResult,
  };
}
