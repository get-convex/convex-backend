import { Value, convexToJson } from "convex/values";
import { useLayoutEffect, useMemo, useState } from "react";
import { useLocalStoreClient } from "./LocalStoreProvider";
import { SyncQueryResult } from "../shared/types";
import { DefaultFunctionArgs } from "convex/server";
import { LocalQuery, LocalMutation } from "./definitionFactory";

export function useLocalQuery<
  T extends Value,
  Args extends DefaultFunctionArgs,
>(
  localQuery: LocalQuery<Args, T>,
  args: Args,
  debugName?: string,
): T | undefined {
  const localClient = useLocalStoreClient();
  const argsJson = JSON.stringify(
    convexToJson((args ?? {}) as unknown as Value),
  );
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const stableArgs = useMemo(() => args, [argsJson]);
  const [result, setResult] = useState<SyncQueryResult>({ kind: "loading" });
  const fn = localQuery.handler;
  const fnDebugName = localQuery.debugName;
  const fullDebugName = [fnDebugName, debugName]
    .filter((d) => d !== undefined)
    .join(":");
  // By using `useLayoutEffect`, we can guarantee that we don't externalize a loading
  // state and cause a flicker if the data is ready locally.
  useLayoutEffect(() => {
    const syncQuerySubscriptionId = localClient.addSyncQuery(
      fn,
      stableArgs,
      (r) => {
        setResult(r);
      },
      fullDebugName,
    );
    return () => {
      localClient.removeSyncQuery(syncQuerySubscriptionId);
    };
  }, [localClient, fn, stableArgs, fullDebugName]);
  if (result.kind === "loaded") {
    if (result.status === "success") {
      return result.value as T;
    } else {
      throw result.error;
    }
  } else {
    return undefined;
  }
}

// This isn't really used right now in favor of `localClient.mutation`
export function useLocalMutation<
  OptUpdateArgs extends DefaultFunctionArgs,
  ServerArgs extends DefaultFunctionArgs,
>(
  mutation: LocalMutation<ServerArgs, OptUpdateArgs>,
): (args: OptUpdateArgs) => Promise<any> {
  const localClient = useLocalStoreClient();
  return async (args: OptUpdateArgs) => {
    await localClient.mutation(mutation, args as any);
  };
}
