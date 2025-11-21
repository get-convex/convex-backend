import { convexToJson, Value } from "../../values/index.js";

export function canonicalizeUdfPath(udfPath: string): string {
  const pieces = udfPath.split(":");
  let moduleName: string;
  let functionName: string;
  if (pieces.length === 1) {
    moduleName = pieces[0];
    functionName = "default";
  } else {
    moduleName = pieces.slice(0, pieces.length - 1).join(":");
    functionName = pieces[pieces.length - 1];
  }
  if (moduleName.endsWith(".js")) {
    moduleName = moduleName.slice(0, -3);
  }
  return `${moduleName}:${functionName}`;
}

/**
 * The serialization here is not stable, these strings never make it outside the client.
 */

/**
 * A string representing the name and arguments of a query.
 *
 * This is used by the {@link BaseConvexClient}.
 *
 * @public
 */
export type QueryToken = string & { __queryToken: true };

/**
 * A string representing the name and arguments of a paginated query.
 *
 * This is a specialized form of QueryToken used for paginated queries.
 */
export type PaginatedQueryToken = QueryToken & { __paginatedQueryToken: true };

export function serializePathAndArgs(
  udfPath: string,
  args: Record<string, Value>,
): QueryToken {
  return JSON.stringify({
    udfPath: canonicalizeUdfPath(udfPath),
    args: convexToJson(args),
  }) as QueryToken;
}

export function serializePaginatedPathAndArgs(
  udfPath: string,
  args: Record<string, Value>, // args WITHOUT paginationOpts
  options: { initialNumItems: number; id: number },
): PaginatedQueryToken {
  const { initialNumItems, id } = options;
  const result = JSON.stringify({
    type: "paginated",
    udfPath: canonicalizeUdfPath(udfPath),
    args: convexToJson(args),
    options: convexToJson({ initialNumItems, id }),
  }) as PaginatedQueryToken;
  return result;
}

export function serializedQueryTokenIsPaginated(
  token: QueryToken | PaginatedQueryToken,
): token is PaginatedQueryToken {
  return JSON.parse(token).type === "paginated";
}
