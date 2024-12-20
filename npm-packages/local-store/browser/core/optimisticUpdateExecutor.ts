import {
  DefaultFunctionArgs,
  FunctionReference,
  SchemaDefinition,
} from "convex/server";
import { IndexRangeRequest, MutationInfo, TableName } from "../../shared/types";
import { Writes } from "./protocol";
import { LoadingError } from "../localDbReader";
import { GenericId, Value } from "convex/values";
import { LocalDbWriterImpl } from "../localDbWriter";
import { CopyOnWriteLocalStore } from "./localStore";
import { SingleIndexRangeExecutor } from "./paginator";
import { LocalDbWriter } from "../../react/localDb";

export type SyncMutation = (ctx: { localDb: any }, args: any) => Value;

export type MutationMap = Record<
  string,
  {
    fn: FunctionReference<"mutation">;
    optimisticUpdate?: (
      ctx: { localDb: LocalDbWriter<any> },
      args: DefaultFunctionArgs,
    ) => void;
  }
>;

export function executeSyncMutation(
  syncSchema: SchemaDefinition<any, any>,
  mutationMap: MutationMap,
  mutationInfo: MutationInfo,
  localStore: CopyOnWriteLocalStore,
): {
  result: "loading" | "success" | "error";
  localStore: CopyOnWriteLocalStore;
} {
  let syncMutationResult: "loading" | "success" | "error" | null = null;
  const syncMutation = mutationMap[mutationInfo.mutationName];
  if (syncMutation === undefined) {
    console.error("Sync mutation not found");
    return {
      result: "error",
      localStore,
    };
  }
  const optimisticUpdate = syncMutation.optimisticUpdate;
  if (optimisticUpdate === undefined) {
    console.warn("Optimistic update for sync mutation not found");
    return {
      result: "success",
      localStore,
    };
  }

  const handleRangeRequest = (rangeRequest: IndexRangeRequest) => {
    const singleIndexRangeExecutor = new SingleIndexRangeExecutor(
      rangeRequest,
      syncSchema,
      localStoreClone,
    );
    const result = singleIndexRangeExecutor.tryFulfill();
    switch (result.state) {
      case "fulfilled":
        return result.results;
      case "waitingOnLoadingPage": {
        syncMutationResult = "loading";
        throw new LoadingError();
      }
      case "needsMorePages": {
        syncMutationResult = "loading";
        throw new LoadingError();
      }
    }
  };

  const loadObject = (tableName: TableName, id: GenericId<any>) => {
    const result = localStore.loadObject(tableName, id);
    if (result === undefined) {
      syncMutationResult = "loading";
      throw new LoadingError();
    }
    return result;
  };

  const localStoreClone = localStore.clone();
  const writes = new Writes();
  const localDb = new LocalDbWriterImpl(
    syncSchema,
    handleRangeRequest,
    loadObject,
    (tableName, id, doc) => {
      writes.set(tableName, id, doc);
      localStoreClone.applyWrites(writes);
    },
  );
  try {
    optimisticUpdate({ localDb: localDb as any }, mutationInfo.optUpdateArgs);
    if (syncMutationResult === null) {
      syncMutationResult = "success";
    }
  } catch (e) {
    if (e instanceof LoadingError) {
      syncMutationResult = "loading";
    } else {
      syncMutationResult = "error";
    }
  }
  return {
    result: syncMutationResult,
    localStore: syncMutationResult === "success" ? localStoreClone : localStore,
  };
}
