import {
  ConvexSubscriptionId,
  IndexName,
  Key,
  MutationInfo,
  PageArguments,
  PageResult,
  PersistId,
  TableName,
} from "../../shared/types";

import { GenericId, convexToJson } from "convex/values";
import {
  MutationId,
  SyncFunction,
  SyncQueryResult,
  SyncQuerySubscriptionId,
} from "../../shared/types";
import { DefaultFunctionArgs, GenericDocument } from "convex/server";
import { SyncQuery } from "./syncQueryExecutor";

export type RangeResponse = Array<GenericDocument>;

export type Page = {
  tableName: TableName;
  indexName: IndexName;
  convexSubscriptionId: ConvexSubscriptionId;
  state:
    | {
        kind: "loaded";
        value: PageResult;
      }
    | {
        kind: "loading";
        target: Key;
      };
};

export class Writes {
  writes: Map<TableName, Map<GenericId<any>, GenericDocument | null>>;
  constructor() {
    this.writes = new Map();
  }

  prettyPrint(): string {
    let result = "";
    for (const [tableName, tableWrites] of this.writes.entries()) {
      result += `${tableName}:\n`;
      for (const [id, doc] of tableWrites.entries()) {
        result += `  ${id}: ${JSON.stringify(convexToJson(doc))}\n`;
      }
    }
    return result;
  }

  set(tableName: TableName, id: GenericId<any>, doc: GenericDocument | null) {
    if (!this.writes.has(tableName)) {
      this.writes.set(tableName, new Map());
    }
    const tableWrites = this.writes.get(tableName)!;
    tableWrites.set(id, doc);
  }

  apply(other: Writes) {
    for (const [tableName, tableWrites] of other.writes) {
      const existingTableWrites = this.writes.get(tableName) ?? new Map();
      for (const [id, write] of tableWrites) {
        existingTableWrites.set(id, write);
      }
      this.writes.set(tableName, existingTableWrites);
    }
  }

  clone(): Writes {
    const clone = new Writes();
    clone.writes = new Map(
      Array.from(this.writes.entries()).map(([tableName, tableWrites]) => [
        tableName,
        new Map(tableWrites.entries()),
      ]),
    );
    return clone;
  }
}

export type CorePersistenceRequest =
  | {
      requestor: "LocalPersistence";
      kind: "ingestFromLocalPersistence";
      pages: Page[];
      serverTs: number;
    }
  | {
      requestor: "LocalPersistence";
      kind: "localPersistComplete";
      persistId: PersistId;
    };

export type CoreRequest =
  | {
      requestor: "UI";
      kind: "addSyncQuerySubscription";
      syncQuerySubscriptionId: SyncQuerySubscriptionId;
      syncQueryFn: SyncQuery;
      syncQueryArgs: DefaultFunctionArgs;
    }
  | {
      requestor: "UI";
      kind: "unsubscribeFromSyncQuery";
      syncQuerySubscriptionId: SyncQuerySubscriptionId;
    }
  | {
      requestor: "UI";
      kind: "mutate";
      mutationInfo: MutationInfo;
    }
  | NetworkTransition
  | {
      requestor: "Network";
      kind: "mutationResponseFromNetwork";
      mutationId: string;
      result: any;
    }
  | CorePersistenceRequest;

export type NetworkTransition = {
  requestor: "Network";
  kind: "transitionFromNetwork";
  serverTs: number;
  queryResults: Map<
    ConvexSubscriptionId,
    | { kind: "success"; result: Page }
    | { kind: "error"; errorMessage: string; errorData: any }
  >;
  reflectedMutations: MutationId[];
};

export type UITransition = {
  recipient: "UI";
  kind: "transition";
  syncQueryUpdates: Map<SyncQuerySubscriptionId, SyncQueryResult>;
  mutationsApplied: Set<MutationId>;
};

export type UINewSyncQuery = {
  recipient: "UI";
  kind: "newSyncQuery";
  syncQuerySubscriptionId: SyncQuerySubscriptionId;
  syncQueryResult: SyncQueryResult;
};

export type CoreResponse =
  | UITransition
  | UINewSyncQuery
  | {
      recipient: "Network";
      kind: "sendMutationToNetwork";
      mutationInfo: MutationInfo;
    }
  | {
      recipient: "Network";
      kind: "sendQueryToNetwork";
      syncFunction: SyncFunction;
      pageRequest: PageArguments;
    }
  | {
      recipient: "Network";
      kind: "removeQueryFromNetwork";
      queriesToRemove: ConvexSubscriptionId[];
    }
  | {
      recipient: "LocalPersistence";
      kind: "persistPages";
      pages: Page[];
      persistId: PersistId;
    }
  | {
      recipient: "LocalPersistence";
      kind: "persistMutation";
      persistId: PersistId;
      mutationInfo: MutationInfo;
    };
