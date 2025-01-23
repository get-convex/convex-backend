import {
  ConvexSubscriptionId,
  MutationId,
  PageResult,
  SyncQueryResult,
  SyncQuerySubscriptionId,
} from "../shared/types";
import {
  DefaultFunctionArgs,
  FunctionReference,
  SchemaDefinition,
  getFunctionName,
} from "convex/server";
import { Driver } from "./driver";
import ReactDOM from "react-dom";
import { Value } from "convex/values";
import { Page, UITransition } from "./core/protocol";
import { parseIndexNameAndTableName } from "../shared/queryTokens";
import { MutationMap } from "./core/optimisticUpdateExecutor";
import { Logger } from "./logger";
import { LocalMutation } from "../react/definitionFactory";

type TransitionState = {
  transitionId: string;

  // We signal that a transition is ready when we receive the corresponding
  // transition message from the core. This signals the base client that it's
  // safe to call the endTransition callback.
  ready: Promise<void>;
  resolveReady: (value: any) => void;

  // A transition completes when its results are applied to the UI.
  complete: Promise<void>;
  resolveComplete: (value: any) => void;
};

/**
 * UI code interfaces with this directly to do things like request sync queries
 * and sync mutations.
 */
export class LocalStoreClient {
  syncSchema: SchemaDefinition<any, any>;
  stagedQueryUpdates: Map<SyncQuerySubscriptionId, SyncQueryResult>;
  syncQueryListeners: Map<
    SyncQuerySubscriptionId,
    (value: SyncQueryResult) => void
  > = new Map();

  driver: Driver;
  inProgressTransition: TransitionState | null = null;
  private logger: Logger;
  mutationIdToStatus: Map<
    MutationId,
    | {
        status: "unresolved";
        resolver: (value: any) => void;
      }
    | {
        status: "reflectedLocallyButWaitingForNetwork";
      }
    | {
        status: "reflected";
      }
    | {
        status: "reflectedOnNetworkButNotLocally";
        resolver: (value: any) => void;
      }
  > = new Map();
  constructor(opts: {
    syncSchema: SchemaDefinition<any, any>;
    mutations: MutationMap;
    driver: Driver;
  }) {
    this.syncSchema = opts.syncSchema;
    this.stagedQueryUpdates = new Map();
    this.driver = opts.driver;
    this.logger = this.driver.logger;
    this.driver.localPersistence.addListener((request) => {
      this.driver.receive(request);
    });
    this.driver.addUiTransitionHandler((t) => {
      this.ingestLocalStoreTransition(t);
    });
    this.driver.addNewSyncQueryResultHandler((r) => {
      this.ingestNewSyncQueryResults(
        new Map([[r.syncQuerySubscriptionId, r.syncQueryResult]]),
      );
    });
    this.driver.network.addOnTransitionHandler((transition) => {
      const transitionId = crypto.randomUUID();
      this.logger.debug("startTransition", transitionId, transition);
      const queryResults: Map<
        ConvexSubscriptionId,
        | { kind: "success"; result: Page }
        | {
            kind: "error";
            errorMessage: string;
            errorData: Value | undefined;
          }
      > = new Map();
      for (const { token, modification } of transition.queries) {
        const parsed = parseIndexNameAndTableName(token);
        if (!parsed) {
          // This isn't a local store query, so skip it
          continue;
        }
        const { indexName, tableName } = parsed;
        if (modification.kind === "Removed") {
          continue;
        }
        const functionResult = modification.result;
        if (functionResult === undefined) {
          throw new Error(
            "Query result is unexpectedly loading for a local store query",
          );
        } else if (functionResult.success === false) {
          const v: {
            kind: "error";
            errorMessage: string;
            errorData: Value | undefined;
          } = {
            kind: "error" as const,
            errorMessage: functionResult.errorMessage,
            errorData: functionResult.errorData,
          };
          queryResults.set(token as ConvexSubscriptionId, v);
        } else {
          queryResults.set(token as ConvexSubscriptionId, {
            kind: "success" as const,
            result: {
              tableName,
              indexName,
              convexSubscriptionId: token as ConvexSubscriptionId,
              state: {
                kind: "loaded" as const,
                value: functionResult.value as unknown as PageResult,
              },
            },
          });
        }
      }
      const reflectedMutationIds: MutationId[] =
        transition.reflectedMutations.flatMap((m) => {
          const mutationId = this.driver.network.getMutationId(m.requestId);
          if (mutationId === null) {
            return [];
          }
          return [mutationId];
        });
      for (const mutationId of reflectedMutationIds) {
        const status = this.mutationIdToStatus.get(mutationId);
        if (status?.status === "unresolved") {
          this.mutationIdToStatus.set(mutationId, {
            status: "reflectedOnNetworkButNotLocally",
            resolver: status.resolver,
          });
        } else if (status?.status === "reflectedLocallyButWaitingForNetwork") {
          this.mutationIdToStatus.set(mutationId, {
            status: "reflected",
          });
        }
      }
      this.driver.receive({
        requestor: "Network",
        kind: "transitionFromNetwork",
        serverTs: 0 as any,
        queryResults,
        reflectedMutations: reflectedMutationIds,
      });
      this.logger.debug("endTransition", transitionId, transition);
      this.updateAllSyncQueries();
    });
  }

  // When we add a new sync query, and get a result for it at the current
  // local store version, we can update the UI immediately

  // We want the to be separate from handling a server transition
  ingestNewSyncQueryResults(
    results: Map<SyncQuerySubscriptionId, SyncQueryResult>,
  ) {
    ReactDOM.unstable_batchedUpdates(() => {
      for (const [syncQueryId, result] of results) {
        const listener = this.syncQueryListeners.get(syncQueryId);
        if (listener) {
          listener(result);
        }
      }
    });
  }

  ingestLocalStoreTransition(transition: UITransition) {
    this.logger.debug(
      "ingestLocalStoreTransition",
      this.inProgressTransition,
      transition,
    );
    // This could be called multiple times per server transition
    // like for optimistic updates
    for (const [syncQueryId, result] of transition.syncQueryUpdates) {
      this.stagedQueryUpdates.set(syncQueryId, result);
    }
    for (const mutationId of transition.mutationsApplied) {
      const status = this.mutationIdToStatus.get(mutationId);
      if (status?.status === "unresolved") {
        status.resolver(null as any);
        this.mutationIdToStatus.set(mutationId, {
          status: "reflectedLocallyButWaitingForNetwork",
        });
      } else if (status?.status === "reflectedOnNetworkButNotLocally") {
        status.resolver(null as any);
        this.mutationIdToStatus.set(mutationId, {
          status: "reflected",
        });
      }
    }
    // If there isn't an in-progress transition, apply all updates immediately.
    if (!this.inProgressTransition) {
      this.updateAllSyncQueries();
      return;
    }
    // If the core is telling us (the UI) to transition, we know that the current in-progress transition
    // has been safely written to persistence, so we can signal to the base client to complete its
    // transition. This will place our end transition callback on the microtask queue.
    this.inProgressTransition.resolveReady(void 0);
  }

  addSyncQuery(
    queryFn: any,
    args: any,
    onUpdate: (
      result: SyncQueryResult,
      syncQuerySubscriptionId: SyncQuerySubscriptionId,
    ) => void,
    debugName?: string,
  ) {
    const syncQuerySubscriptionId = `${
      debugName ?? "syncQuery"
    }:${crypto.randomUUID()}` as SyncQuerySubscriptionId;
    this.syncQueryListeners.set(syncQuerySubscriptionId, (result) =>
      onUpdate(result, syncQuerySubscriptionId),
    );
    this.driver.receive({
      requestor: "UI",
      kind: "addSyncQuerySubscription",
      syncQuerySubscriptionId,
      syncQueryFn: queryFn,
      syncQueryArgs: args,
    });
    return syncQuerySubscriptionId;
  }

  removeSyncQuery(syncQuerySubscriptionId: SyncQuerySubscriptionId) {
    this.driver.receive({
      requestor: "UI",
      kind: "unsubscribeFromSyncQuery",
      syncQuerySubscriptionId,
    });
  }

  updateAllSyncQueries() {
    // Grab the staged query updates and call a bunch of setState things in a batch
    // Then clear the staged query updates
    ReactDOM.unstable_batchedUpdates(() => {
      this.logger.debug("updateAllSyncQueries", this.stagedQueryUpdates);
      for (const [syncQueryId, result] of this.stagedQueryUpdates) {
        const listener = this.syncQueryListeners.get(syncQueryId);
        if (listener) {
          listener(result);
        }
      }
    });
    this.stagedQueryUpdates.clear();
  }

  mutation<
    ServerArgs extends DefaultFunctionArgs,
    OptimisticUpdateArgs extends DefaultFunctionArgs,
  >(
    mutation: LocalMutation<ServerArgs, OptimisticUpdateArgs>,
    args: OptimisticUpdateArgs,
  ): Promise<any> {
    const { mutationPromise } = this.mutationInternal(
      mutation.fn,
      args,
      mutation.serverArgs(args),
    );
    return mutationPromise;
  }

  mutationInternal(
    fn: FunctionReference<"mutation">,
    optUpdateArgs: DefaultFunctionArgs,
    serverArgs: DefaultFunctionArgs,
  ) {
    const mutationId = crypto.randomUUID() as MutationId;
    const mutationPromise = new Promise((resolve) => {
      this.mutationIdToStatus.set(mutationId, {
        status: "unresolved",
        resolver: resolve,
      });
    });
    this.driver.receive({
      requestor: "UI",
      kind: "mutate",
      mutationInfo: {
        mutationId,
        mutationPath: fn,
        mutationName: getFunctionName(fn),
        optUpdateArgs,
        serverArgs,
      },
    });
    return { mutationPromise, mutationId };
  }

  getMutationStatus(mutationId: MutationId) {
    return this.mutationIdToStatus.get(mutationId);
  }

  async waitForTransitionToComplete(): Promise<void> {
    if (this.inProgressTransition === null) {
      return;
    }
    await this.inProgressTransition.complete;
  }
}
