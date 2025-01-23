import { FunctionReference, getFunctionName } from "convex/server";
import { ConvexSubscriptionId, IndexName, TableName } from "./types";
import { convexToJson } from "convex/values";
import { Value } from "convex/values";

export function createQueryToken(
  path: FunctionReference<any, any, any, any>,
  args: any,
): ConvexSubscriptionId {
  return serializePathAndArgs(
    getFunctionName(path),
    args,
  ) as ConvexSubscriptionId;
}

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
 * A string representing the name and arguments of a query.
 *
 * This is used by the {@link BaseConvexClient}.
 *
 * @public
 */
export type QueryToken = string;

export function serializePathAndArgs(
  udfPath: string,
  args: Record<string, Value>,
): QueryToken {
  return JSON.stringify({
    udfPath: canonicalizeUdfPath(udfPath),
    args: convexToJson(args),
  });
}

export const parseIndexNameAndTableName = (
  token: QueryToken,
): {
  indexName: IndexName;
  tableName: TableName;
} | null => {
  const { udfPath } = JSON.parse(token);
  const [filePath, indexName] = udfPath.split(":");
  const filePathParts = filePath.split("/");
  if (filePathParts.length < 2) {
    return null;
  }
  const tableName = filePathParts.at(-1);
  const syncPart = filePathParts.at(-2);
  if (syncPart !== "sync") {
    return null;
  }
  return { indexName, tableName };
};
