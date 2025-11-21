import type { PaginationResult } from "../../server/index.js";
import type { Infer, Value } from "../../values/index.js";
import type { paginationOptsValidator } from "../../server/index.js";

export type PaginationStatus =
  | "LoadingFirstPage"
  | "CanLoadMore"
  | "LoadingMore"
  | "Exhausted";

export type PaginatedQueryResult<T> = {
  results: T[];
  status: PaginationStatus;
  loadMore: LoadMoreOfPaginatedQuery;
};

/**
 * Returns whether loading more was actually initiated; in cases where
 * a paginated query is already loading more items or there are no more
 * items available, calling loadMore() may do nothing.
 */
export type LoadMoreOfPaginatedQuery = (numItems: number) => boolean;

// The arguments for each page query.
export function asPaginationArgs(value: Value): Record<string, Value> & {
  paginationOpts: Infer<typeof paginationOptsValidator>;
} {
  if (typeof (value as any).paginationOpts.numItems !== "number") {
    throw new Error(`Not valid paginated query args: ${JSON.stringify(value)}`);
  }
  return value as unknown as Record<string, Value> & {
    paginationOpts: Infer<typeof paginationOptsValidator>;
  };
}

/**
 * Validates that a Value is a valid pagination result and returns it cast to PaginationResult.
 */
export function asPaginationResult(value: Value): PaginationResult<Value> {
  if (
    typeof value !== "object" ||
    value === null ||
    !Array.isArray((value as any).page) ||
    typeof (value as any).isDone !== "boolean" ||
    typeof (value as any).continueCursor !== "string"
  ) {
    throw new Error(`Not a valid paginated query result: ${value?.toString()}`);
  }
  return value as unknown as PaginationResult<Value>;
}
