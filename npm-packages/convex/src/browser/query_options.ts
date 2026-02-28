/**
 * Query options are a potential new API for a variety of functions, but in particular a new overload of the React hook for queries.
 *
 * Inspired by https://tanstack.com/query/v5/docs/framework/react/guides/query-options
 */
import type { FunctionArgs, FunctionReference } from "../server/api.js";

/**
 * Query options.
 */
export type QueryOptions<Query extends FunctionReference<"query">> = {
  /**
   * The query function to run.
   */
  query: Query;
  /**
   * The arguments to the query function.
   */
  args: FunctionArgs<Query>;
};

// This helper helps more once we have more inference happening.
export function convexQueryOptions<Query extends FunctionReference<"query">>(
  options: QueryOptions<Query>,
) {
  return options;
}
