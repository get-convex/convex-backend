import {
  QueryCache,
  QueryClient,
  QueryFunction,
  QueryFunctionContext,
  QueryKey,
  UseQueryOptions,
  UseSuspenseQueryOptions,
  hashKey,
  notifyManager,
} from "@tanstack/react-query";
import { ConvexHttpClient } from "convex/browser";
import {
  ConvexReactClient,
  ConvexReactClientOptions,
  Watch,
} from "convex/react";
import {
  FunctionArgs,
  FunctionReference,
  FunctionReturnType,
  getFunctionName,
} from "convex/server";
import { convexToJson } from "convex/values";

// Re-export React Query-friendly names for Convex hooks.
// Never importing "convex/react" from application code should
// prevent import completion of the Convex `useQuery`.
export {
  useQuery as useConvexQuery,
  useQueries as useConvexQueries,
  usePaginatedQuery as useConvexPaginatedQuery,
  useMutation as useConvexMutation,
  useAction as useConvexAction,
  useConvex,
  useConvexAuth,
  optimisticallyUpdateValueInPaginatedQuery,
} from "convex/react";

const isServer = typeof window === "undefined";

function isConvexSkipped(
  queryKey: readonly any[],
): queryKey is ["convexQuery" | "convexAction", unknown, "skip"] {
  return (
    queryKey.length >= 2 &&
    ["convexQuery", "convexAction"].includes(queryKey[0]) &&
    queryKey[2] === "skip"
  );
}

function isConvexQuery(
  queryKey: readonly any[],
): queryKey is [
  "convexQuery",
  FunctionReference<"query">,
  Record<string, any>,
  {},
] {
  return queryKey.length >= 2 && queryKey[0] === "convexQuery";
}

function isConvexAction(
  queryKey: readonly any[],
): queryKey is [
  "convexAction",
  FunctionReference<"action">,
  Record<string, any>,
  {},
] {
  return queryKey.length >= 2 && queryKey[0] === "convexAction";
}

function hash(
  queryKey: [
    "convexQuery",
    FunctionReference<"query">,
    Record<string, any>,
    {},
  ],
): string {
  return `convexQuery|${getFunctionName(queryKey[1])}|${JSON.stringify(
    convexToJson(queryKey[2]),
  )}`;
}

export interface ConvexQueryClientOptions extends ConvexReactClientOptions {
  /** queryClient can also be set later by calling .connect(ReactqueryClient) */
  queryClient?: QueryClient;
}

/**
 * Subscribes to events from a TanStack Query QueryClient and populates query
 * results in it for all Convex query function subscriptions.
 */
export class ConvexQueryClient {
  convexClient: ConvexReactClient;
  subscriptions: Record<
    string, // queryKey hash
    {
      watch: Watch<any>;
      unsubscribe: () => void;
      queryKey: [
        convexKey: "convexQuery",
        func: FunctionReference<"query">,
        args: Record<string, any>,
        options?: {},
      ];
    }
  >;
  unsubscribe: (() => void) | undefined;
  // Only exists during SSR
  serverHttpClient?: ConvexHttpClient;
  _queryClient: QueryClient | undefined;
  get queryClient() {
    if (!this._queryClient) {
      throw new Error(
        "ConvexQueryClient not connected to TanStack QueryClient.",
      );
    }
    return this._queryClient;
  }
  constructor(
    /** A ConvexReactClient instance or a URL to use to instantiate one. */
    client: ConvexReactClient | string,
    /** Options mostly for the ConvexReactClient to be constructed. */
    options: ConvexQueryClientOptions = {},
  ) {
    if (typeof client === "string") {
      this.convexClient = new ConvexReactClient(client, options);
    } else {
      this.convexClient = client as ConvexReactClient;
    }
    this.subscriptions = {};
    if (options.queryClient) {
      this._queryClient = options.queryClient;
      this.unsubscribe = this.subscribeInner(
        options.queryClient.getQueryCache(),
      );
    }
    if (isServer) {
      this.serverHttpClient = new ConvexHttpClient(
        // TODO use .url once convex@1.14 is released
        (this.convexClient as any).address as string,
      );
    }
  }
  /** Complete initialization of ConvexQueryClient by connecting a TanStack QueryClient */
  connect(queryClient: QueryClient) {
    if (this.unsubscribe) {
      throw new Error("already subscribed!");
    }
    this._queryClient = queryClient;
    this.unsubscribe = this.subscribeInner(queryClient.getQueryCache());
  }

  /** Update every query key. Probably not useful, don't use this. */
  onUpdate = () => {
    // Fortunately this does not reset the gc time.
    notifyManager.batch(() => {
      for (const key of Object.keys(this.subscriptions)) {
        this.onUpdateQueryKeyHash(key);
      }
    });
  };
  onUpdateQueryKeyHash(queryHash: string) {
    const subscription = this.subscriptions[queryHash];
    if (!subscription) {
      // If we have no record of this subscription that should be a logic error.
      throw new Error(
        `Internal ConvexQueryClient error: onUpdateQueryKeyHash called for ${queryHash}`,
      );
    }

    const queryCache = this.queryClient.getQueryCache();
    const query = queryCache.get(queryHash);
    if (!query) return;

    const { queryKey, watch } = subscription;
    let result: { ok: true; value: any } | { ok: false; error: unknown };
    try {
      result = { ok: true, value: watch.localQueryResult() };
    } catch (error) {
      result = { ok: false, error };
    }

    if (result.ok) {
      const value = result.value;
      this.queryClient.setQueryData(queryKey, (prev) => {
        if (prev === undefined) {
          // If `prev` is undefined there is no react-query entry for this query key.
          // Return `undefined` to signal not to create one.
          return undefined;
        }
        return value;
      });
    } else {
      const { error } = result;
      // TODO This may not be a stable API. Devtools work this way so it's at
      // least used elsewhere. Either trigger a query by invalidating this query
      // (only feasible if guaranteed to update before the next tick) or
      // look into a `QueryClient.setQueryError` API.
      query?.setState(
        {
          error: error as Error,
          errorUpdateCount: query.state.errorUpdateCount + 1,
          errorUpdatedAt: Date.now(),
          fetchFailureCount: query.state.fetchFailureCount + 1,
          fetchFailureReason: error as Error,
          fetchStatus: "idle",
          status: "error",
        },
        { meta: "set by ConvexQueryClient" },
      );
    }
  }

  subscribeInner(queryCache: QueryCache): () => void {
    if (isServer) return () => {};
    return queryCache.subscribe((event) => {
      if (!isConvexQuery(event.query.queryKey)) {
        return;
      }
      if (isConvexSkipped(event.query.queryKey)) {
        return;
      }

      switch (event.type) {
        // A query has been GC'd so no stale value will be available.
        // In Convex this means we should unsubscribe.
        case "removed": {
          this.subscriptions[event.query.queryHash].unsubscribe();
          delete this.subscriptions[event.query.queryHash];
          break;
        }
        // A query has been requested for the first time.
        // Subscribe to the query so we hold on to it.
        case "added": {
          // There exists only one watch per subscription; but
          // watches are stateless anyway, they're just util code.
          const [_, func, args, _opts] = event.query.queryKey as [
            "convexQuery",
            FunctionReference<"query">,
            any,
            {},
          ];
          const watch = this.convexClient.watchQuery(
            func,
            args,
            // TODO pass journals through
            {},
          );
          const unsubscribe = watch.onUpdate(() => {
            this.onUpdateQueryKeyHash(event.query.queryHash);
          });

          this.subscriptions[event.query.queryHash] = {
            queryKey: event.query.queryKey,
            watch,
            unsubscribe,
          };
          break;
        }
        // Runs when a useQuery mounts
        case "observerAdded": {
          break;
        }
        // Runs when a useQuery unmounts
        case "observerRemoved": {
          if (event.query.getObserversCount() === 0) {
            // The last useQuery subscribed to this query has unmounted.
            // But don't clean up yet, after gcTime a "removed" event
            // will notify that it's time to drop the subscription to
            // the Convex backend.
          }
          break;
        }
        // Fires once per useQuery hook
        case "observerResultsUpdated": {
          break;
        }
        case "updated": {
          if (
            event.action.type === "setState" &&
            event.action.setStateOptions?.meta === "set by ConvexQueryClient"
          ) {
            // This one was caused by us. This may be important to know for
            // breaking infinite loops in the future.
            break;
          }
          break;
        }
        case "observerOptionsUpdated": {
          // observerOptionsUpdated, often because of an unmemoized query key
          break;
        }
      }
    });
  }

  /**
   * Returns a promise for the query result of a query key containing
   * `['convexQuery', FunctionReference, args]` and subscribes via WebSocket
   * to future updates.
   *
   * You can provide a custom fetch function for queries that are not
   * Convex queries.
   */
  queryFn(
    otherFetch: QueryFunction<unknown, QueryKey> = throwBecauseNotConvexQuery,
  ) {
    return async <
      ConvexQueryReference extends FunctionReference<"query", "public">,
    >(
      context: QueryFunctionContext<ReadonlyArray<unknown>>,
    ): Promise<FunctionReturnType<ConvexQueryReference>> => {
      if (isConvexSkipped(context.queryKey)) {
        throw new Error(
          "Skipped query should not actually be run, should { enabled: false }",
        );
      }
      // Only queries can be requested consistently (at a previous timestamp),
      // actions and mutations run at the latest timestamp.
      if (isConvexQuery(context.queryKey)) {
        const [_, func, args] = context.queryKey;
        if (isServer) {
          return await this.serverHttpClient!.consistentQuery(func, args);
        } else {
          return await this.convexClient.query(func, args);
        }
      }
      if (isConvexAction(context.queryKey)) {
        const [_, func, args] = context.queryKey;
        if (isServer) {
          const client = new ConvexHttpClient(
            // TODO expose this private property
            (this.convexClient as any).address as string,
          );
          const data = await client.action(func, args);
          return data;
        } else {
          const data = await this.convexClient.action(func, args);
          return data;
        }
      }
      return otherFetch(context);
    };
  }

  /**
   * Set this globally to use Convex query functions.
   *
   * ```ts
   * const queryClient = new QueryClient({
   *   defaultOptions: {
   *    queries: {
   *       queryKeyHashFn: convexQueryClient.hashFn(),
   *     },
   *   },
   * });
   *
   * You can provide a custom hash function for keys that are not for Convex
   * queries.
   */
  hashFn(otherHashKey: (queryKey: ReadonlyArray<unknown>) => string = hashKey) {
    return (queryKey: ReadonlyArray<unknown>) => {
      if (isConvexQuery(queryKey)) {
        return hash(queryKey);
      }
      return otherHashKey(queryKey);
    };
  }

  /**
   * Query options factory for Convex query function subscriptions.
   *
   * ```
   * useQuery(client.queryOptions(api.foo.bar, args))
   * ```
   *
   * If you need to specify other options spread it:
   * ```
   * useQuery({
   *   ...convexQueryClient.queryOptions(api.foo.bar, args),
   *   placeholderData: { name: "me" }
   * });
   * ```
   */
  queryOptions = <ConvexQueryReference extends FunctionReference<"query">>(
    funcRef: ConvexQueryReference,
    queryArgs: FunctionArgs<ConvexQueryReference>,
  ): Pick<
    UseQueryOptions<
      FunctionReturnType<ConvexQueryReference>,
      Error,
      FunctionReturnType<ConvexQueryReference>,
      ["convexQuery", ConvexQueryReference, FunctionArgs<ConvexQueryReference>]
    >,
    "queryKey" | "queryFn" | "staleTime"
  > => {
    return {
      queryKey: [
        "convexQuery",
        // Make query key serializable
        getFunctionName(funcRef) as unknown as typeof funcRef,
        // TODO bigints are not serializable
        queryArgs,
      ],
      queryFn: this.queryFn(),
      staleTime: Infinity,
      // We cannot set hashFn here, see
      // https://github.com/TanStack/query/issues/4052#issuecomment-1296174282
      // so the developer must set it globally.
    };
  };
}

/**
 * Query options factory for Convex query function subscriptions.
 * This options factory requires the `convexQueryClient.queryFn()` has been set
 * as the default `queryFn` globally.
 *
 * ```
 * useQuery(convexQuery(api.foo.bar, args))
 * ```
 *
 * If you need to specify other options spread it:
 * ```
 * useQuery({
 *   ...convexQuery(api.messages.list, { channel: 'dogs' }),
 *   placeholderData: [{ name: "Snowy" }]
 * });
 * ```
 */
export const convexQuery = <
  ConvexQueryReference extends FunctionReference<"query">,
  Args extends FunctionArgs<ConvexQueryReference> | "skip",
>(
  funcRef: ConvexQueryReference,
  queryArgs: Args,
): Args extends "skip"
  ? Pick<
      UseQueryOptions<
        FunctionReturnType<ConvexQueryReference>,
        Error,
        FunctionReturnType<ConvexQueryReference>,
        [
          "convexQuery",
          ConvexQueryReference,
          FunctionArgs<ConvexQueryReference>,
        ]
      >,
      "queryKey" | "queryFn" | "staleTime" | "enabled"
    >
  : Pick<
      UseSuspenseQueryOptions<
        FunctionReturnType<ConvexQueryReference>,
        Error,
        FunctionReturnType<ConvexQueryReference>,
        [
          "convexQuery",
          ConvexQueryReference,
          FunctionArgs<ConvexQueryReference>,
        ]
      >,
      "queryKey" | "queryFn" | "staleTime"
    > => {
  return {
    queryKey: [
      "convexQuery",
      // Make query key serializable
      getFunctionName(funcRef) as unknown as typeof funcRef,
      // TODO bigints are not serializable
      queryArgs === "skip" ? "skip" : queryArgs,
    ],
    staleTime: Infinity,
    ...(queryArgs === "skip" ? { enabled: false } : {}),
  };
};

/**
 * Query options factory for Convex action function.
 * Not that Convex actions are live updating: they follow the normal react-query
 * semantics of refreshing on
 *
 * ```
 * useQuery(convexQuery(api.weather.now, { location: "SF" }))
 * ```
 *
 * If you need to specify other options spread it:
 * ```
 * useQuery({
 *   ...convexAction(api.weather.now, { location: "SF" }),
 *   placeholderData: { status: "foggy and cool" }
 * });
 * ```
 */
export const convexAction = <
  ConvexActionReference extends FunctionReference<"action">,
  Args extends FunctionArgs<ConvexActionReference> | "skip",
>(
  funcRef: ConvexActionReference,
  args: Args,
): Args extends "skip"
  ? Pick<
      UseQueryOptions<
        FunctionReturnType<ConvexActionReference>,
        Error,
        FunctionReturnType<ConvexActionReference>,
        [
          "convexAction",
          ConvexActionReference,
          FunctionArgs<ConvexActionReference>,
        ]
      >,
      "queryKey" | "queryFn" | "staleTime" | "enabled"
    >
  : Pick<
      UseSuspenseQueryOptions<
        FunctionReturnType<ConvexActionReference>,
        Error,
        FunctionReturnType<ConvexActionReference>,
        [
          "convexAction",
          ConvexActionReference,
          FunctionArgs<ConvexActionReference>,
        ]
      >,
      "queryKey" | "queryFn" | "staleTime"
    > => {
  return {
    queryKey: [
      "convexAction",
      // Make query key serializable
      getFunctionName(funcRef) as unknown as typeof funcRef,
      // TODO bigints are not serializable
      args === "skip" ? {} : args,
    ],
    ...(args === "skip" ? { enabled: false } : {}),
  };
};

function throwBecauseNotConvexQuery(
  context: QueryFunctionContext<ReadonlyArray<unknown>>,
) {
  throw new Error("Query key is not for a Convex Query: " + context.queryKey);
}
