import { QueryToken } from "convex/browser";
import {
  DefaultFunctionArgs,
  FunctionReference,
  GenericDocument,
} from "convex/server";
import { Value } from "convex/values";
import { assert } from "./assert";

export type ConvexSubscriptionId = QueryToken & {
  __brand: "ConvexSubscriptionId";
};

export type SyncQuerySubscriptionId = string & {
  __brand: "SyncQuerySubscriptionId";
};

export type SyncQueryResult =
  | {
      kind: "loaded";
      status: "success";
      value: Value;
    }
  | {
      kind: "loaded";
      status: "error";
      error: any;
    }
  | { kind: "loading" };

export type SyncQueryExecutionId = number & { __brand: "SyncQueryExecutionId" };

export type PersistId = string & { __brand: "PersistId" };

export type MutationResult =
  | {
      status: "success";
      value: Value;
    }
  | {
      status: "error";
      error: any;
    };

export type MutationInfo = {
  mutationName: string;
  mutationId: MutationId;
  mutationPath: FunctionReference<"mutation">;
  optUpdateArgs: DefaultFunctionArgs;
  serverArgs: DefaultFunctionArgs;
};

export type ServerStoreVersion = number & { __brand: "ServerStoreVersion" };

export class LocalStoreVersion {
  private version: number;
  serverVersion: ServerStoreVersion;
  constructor(version: number, serverVersion: ServerStoreVersion) {
    this.version = version;
    this.serverVersion = serverVersion;
  }

  increment(): LocalStoreVersion {
    return new LocalStoreVersion(this.version + 1, this.serverVersion);
  }

  advanceToServerVersion(serverVersion: ServerStoreVersion): LocalStoreVersion {
    assert(
      serverVersion > this.serverVersion,
      "Cannot advance to a past server version",
    );
    return new LocalStoreVersion(this.version + 1, serverVersion);
  }
}

export type MutationId = string & { __brand: "MutationId" };

export type TableName = string;
export type IndexName = string;

// has all of the fields of the index
export type IndexKey = ReadonlyArray<any>;

export type IndexPrefix = ReadonlyArray<any>;

export const MAXIMAL_KEY = {
  kind: "successor",
  value: [],
} as const;

export const MINIMAL_KEY = {
  kind: "predecessor",
  value: [],
} as const;

export type ExactKey = {
  kind: "exact";
  value: IndexKey;
};

export type Key =
  | {
      kind: "successor" | "predecessor";
      value: IndexPrefix;
    }
  | ExactKey;

export type PageArguments = {
  syncTableName: string;
  index: string;
  target: Key;
  log2PageSize: number;
};

export type PageResult = {
  results: GenericDocument[];
  lowerBound: LowerBound;
  upperBound: UpperBound;
};

export type LowerBound =
  | { kind: "successor"; value: IndexPrefix }
  | typeof MINIMAL_KEY;
export type UpperBound = ExactKey | typeof MAXIMAL_KEY;

export type IndexRangeBounds = {
  lowerBound: IndexPrefix; // [conversationId]
  lowerBoundInclusive: boolean;
  upperBound: IndexPrefix; // [conversationId]
  upperBoundInclusive: boolean;
};

// Output cursor from the developer-defined generator function.
export type GeneratorCursor =
  | typeof MAXIMAL_KEY
  | typeof MINIMAL_KEY
  | ExactKey;
// Input cursor to the developer-defined generator function.
// In addition to being an index key or minimal/maximal key, it can also be an inclusive or exclusive bound.
export type GeneratorInputCursor = {
  key: IndexPrefix;
  inclusive: boolean;
};

export const isMaximal = (c: Key): c is typeof MAXIMAL_KEY => {
  return c.kind === "successor" && c.value.length === 0;
};

export const isMinimal = (c: Key): c is typeof MINIMAL_KEY => {
  return c.kind === "predecessor" && c.value.length === 0;
};

export const isExact = (c: Key): c is ExactKey => {
  return c.kind === "exact";
};

// Corresponds to a `db.query`
export type PaginatorSubscriptionId = string & {
  __brand: "PaginatorSubscriptionId";
};

export type SyncFunction = FunctionReference<
  "query",
  "public",
  PageArguments,
  PageResult
>;

export type SyncGetFunction = FunctionReference<
  "query",
  "public",
  { _id: string },
  GenericDocument | null
>;

export type IndexRangeRequest = {
  tableName: TableName;
  indexName: IndexName;
  count: number;
  indexRangeBounds: IndexRangeBounds;
  order: "asc" | "desc";
};
