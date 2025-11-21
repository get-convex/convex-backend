import {
  FunctionArgs,
  FunctionReference,
  makeFunctionReference,
} from "../server/api.js";
import { jsonToConvex } from "../values/index.js";
import type { Preloaded } from "./hydration.js";

/**
 * Parse a preloaded query payload into its constituent parts.
 *
 * This is a hook-free helper that can be used by both `useQuery` and
 * `usePreloadedQuery` to avoid duplicating the parsing logic.
 *
 * @internal
 */
export function parsePreloaded<Query extends FunctionReference<"query">>(
  preloaded: Preloaded<Query>,
): {
  queryReference: Query;
  argsObject: FunctionArgs<Query>;
  preloadedResult: Query["_returnType"];
} {
  return {
    queryReference: makeFunctionReference(preloaded._name) as Query,
    argsObject: jsonToConvex(preloaded._argsJSON) as FunctionArgs<Query>,
    preloadedResult: jsonToConvex(preloaded._valueJSON) as Query["_returnType"],
  };
}
