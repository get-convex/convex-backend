import {
  SyncQuerySubscriptionId,
  ConvexSubscriptionId,
  LocalStoreVersion,
  SyncQueryResult,
  PersistId,
  MutationResult,
  MutationInfo,
  MutationId,
  ServerStoreVersion,
} from "../../shared/types";
import { DefaultFunctionArgs, SchemaDefinition } from "convex/server";
import { CoreRequest, Page } from "./protocol";
import { CoreResponse } from "./protocol";
import { SyncQueryManager } from "./syncQueryManager";
import { CopyOnWriteLocalStore } from "./localStore";
import { executeSyncQuery, SyncQuery } from "./syncQueryExecutor";
import { MutationMap, executeSyncMutation } from "./optimisticUpdateExecutor";
import { Logger } from "../logger";

/**
 * Deterministic synchronous local store.
 * Interacts via `receive`, which accepts `CoreRequest`s and returns `CoreResponse`s.
 */
type Transition = {
  mutationsAddedAccordingToNetwork: Array<MutationInfo>;
  mutationsReflectedAccordingToNetwork: Array<MutationId>;
  pagesChanged: Map<ConvexSubscriptionId, Page>;
};

export class CoreSyncEngine {
  private syncQueries: Map<
    SyncQuerySubscriptionId,
    {
      args: any;
      fn: SyncQuery;
    }
  > = new Map();

  private syncQueryManager: SyncQueryManager;
  private transitions: Array<Transition> = [];

  private log2PageSize: number = 4;

  private subscriptions: Set<ConvexSubscriptionId> = new Set();
  private mutationsAwaitingLocalPersistence: Map<PersistId, MutationInfo> =
    new Map();
  private mutationsUnreflectedAccordingToNetwork: Array<MutationInfo> = [];
  private mutationResults: Map<string, MutationResult> = new Map();

  private receivedInitialPages: boolean = false;
  private deferredTransitions: Array<Transition> = [];

  constructor(
    private schema: SchemaDefinition<any, any>,
    private mutationMap: MutationMap,
    private logger: Logger,
  ) {
    this.syncQueryManager = new SyncQueryManager({
      localStoreVersion: new LocalStoreVersion(0, 0 as ServerStoreVersion),
      store: new CopyOnWriteLocalStore(schema),
    });
  }

  receive(syncRequest: CoreRequest): CoreResponse[] {
    // always try and advance snapshots
    // always try and unsubscribe
    const { responses, tryAdvanceSnapshots, tryUnsubscribe } =
      this._handleRequest(syncRequest);
    const outputs: CoreResponse[] = [...responses];
    if (tryAdvanceSnapshots) {
      outputs.push(...this.advanceSnapshots());
    }
    if (tryUnsubscribe) {
      outputs.push(...this.unsubscribeFromPageQueries());
    }
    return outputs;
  }

  private _handleRequest(syncRequest: CoreRequest): {
    responses: CoreResponse[];
    tryAdvanceSnapshots: boolean;
    tryUnsubscribe: boolean;
  } {
    switch (syncRequest.kind) {
      case "addSyncQuerySubscription": {
        const responses = this.handleAddSyncQuerySubscription(
          syncRequest.syncQuerySubscriptionId,
          syncRequest.syncQueryFn,
          syncRequest.syncQueryArgs,
        );
        return {
          responses,
          tryAdvanceSnapshots: false,
          tryUnsubscribe: false,
        };
      }
      case "unsubscribeFromSyncQuery": {
        const responses = this.handleUnsubscribeFromSyncQuery(
          syncRequest.syncQuerySubscriptionId,
        );
        return {
          responses,
          tryAdvanceSnapshots: true,
          tryUnsubscribe: true,
        };
      }
      case "ingestFromLocalPersistence": {
        if (this.receivedInitialPages) {
          throw new Error("Received initial pages twice");
        }
        this.receivedInitialPages = true;

        const { pages } = syncRequest;

        // TODO -- ingest mutations from local persistence
        console.log(`Loaded ${pages.length} pages from local persistence.`);
        this.transitions.push({
          pagesChanged: new Map(
            pages.map((page) => [page.convexSubscriptionId, page]),
          ),
          mutationsAddedAccordingToNetwork: [],
          mutationsReflectedAccordingToNetwork: [],
        });
        const responses: CoreResponse[] = [];
        for (const transition of this.deferredTransitions) {
          this.transitions.push(transition);
          const persistId = crypto.randomUUID() as PersistId;
          responses.push({
            recipient: "LocalPersistence",
            kind: "persistPages",
            pages: Array.from(transition.pagesChanged.values()),
            persistId,
          });
        }
        this.deferredTransitions = [];
        return {
          responses,
          tryAdvanceSnapshots: true,
          tryUnsubscribe: false,
        };
      }
      case "mutate": {
        const { mutationInfo } = syncRequest;
        const persistId = crypto.randomUUID() as PersistId;
        this.mutationsAwaitingLocalPersistence.set(persistId, mutationInfo);
        // const responses: CoreResponse[] = [
        //   {
        //     recipient: "Network",
        //     kind: "sendMutationToNetwork",
        //     mutationPath,
        //     args,
        //     mutationId,
        //   },
        // ];
        const responses: CoreResponse[] = [
          {
            recipient: "LocalPersistence",
            kind: "persistMutation",
            persistId,
            mutationInfo,
          },
        ];
        return {
          responses,
          tryAdvanceSnapshots: false,
          tryUnsubscribe: false,
        };
      }
      case "localPersistComplete": {
        const { persistId } = syncRequest;
        const responses: CoreResponse[] = [];
        let tryAdvanceSnapshots = false;
        const mutationInfo =
          this.mutationsAwaitingLocalPersistence.get(persistId);
        if (mutationInfo !== undefined) {
          this.transitions.push({
            mutationsAddedAccordingToNetwork: [mutationInfo],
            mutationsReflectedAccordingToNetwork: [],
            pagesChanged: new Map(),
          });
          responses.push({
            recipient: "Network",
            kind: "sendMutationToNetwork",
            mutationInfo,
          });
          tryAdvanceSnapshots = true;
        }
        return {
          responses,
          tryAdvanceSnapshots,
          tryUnsubscribe: false,
        };
      }
      case "mutationResponseFromNetwork": {
        const { mutationId, result } = syncRequest;
        this.mutationResults.set(mutationId, result);
        return {
          responses: [],
          tryAdvanceSnapshots: false,
          tryUnsubscribe: false,
        };
      }
      case "transitionFromNetwork": {
        const { queryResults, reflectedMutations } = syncRequest;
        const persistId = crypto.randomUUID() as PersistId;
        const transition: Transition = {
          pagesChanged: new Map(
            Array.from(queryResults.entries()).flatMap(
              ([convexSubscriptionId, result]) =>
                result.kind === "success"
                  ? [[convexSubscriptionId, result.result]]
                  : [],
            ),
          ),
          mutationsAddedAccordingToNetwork: [],
          mutationsReflectedAccordingToNetwork: reflectedMutations,
        };
        if (!this.receivedInitialPages) {
          this.deferredTransitions.push(transition);
          return {
            responses: [],
            tryAdvanceSnapshots: false,
            tryUnsubscribe: false,
          };
        }
        this.transitions.push(transition);
        const responses: CoreResponse[] = [
          {
            recipient: "LocalPersistence",
            kind: "persistPages",
            pages: Array.from(transition.pagesChanged.values()),
            persistId,
          },
        ];
        return {
          responses,
          tryAdvanceSnapshots: true,
          tryUnsubscribe: false,
        };
      }
    }
    const _typecheck: never = syncRequest;
    throw new Error("Unreachable");
  }

  private getCollapsedTransition(): {
    pagesChanged: Map<ConvexSubscriptionId, Page>;
    mutationsAddedAccordingToNetwork: Array<MutationInfo>;
    mutationsReflectedAccordingToNetwork: Array<MutationId>;
  } | null {
    const transition = this.transitions.shift();
    if (transition === undefined) {
      return null;
    }
    let nextTransition = this.transitions.shift();
    while (nextTransition !== undefined) {
      nextTransition.pagesChanged.forEach((page, convexSubscriptionId) => {
        transition.pagesChanged.set(convexSubscriptionId, page);
      });
      for (const mutationInfo of nextTransition.mutationsAddedAccordingToNetwork) {
        transition.mutationsAddedAccordingToNetwork.push(mutationInfo);
      }
      for (const mutationId of nextTransition.mutationsReflectedAccordingToNetwork) {
        const addedIndex =
          transition.mutationsAddedAccordingToNetwork.findIndex(
            (mutationInfo) => mutationInfo.mutationId === mutationId,
          );
        if (addedIndex === -1) {
          transition.mutationsAddedAccordingToNetwork = [
            ...transition.mutationsAddedAccordingToNetwork.slice(0, addedIndex),
            ...transition.mutationsAddedAccordingToNetwork.slice(
              addedIndex + 1,
            ),
          ];
        } else {
          transition.mutationsReflectedAccordingToNetwork.push(mutationId);
        }
      }
      nextTransition = this.transitions.shift();
    }
    return transition;
  }

  private advanceSnapshots(): CoreResponse[] {
    const transition = this.getCollapsedTransition();
    if (transition === null) {
      return [];
    }
    const {
      pagesChanged,
      mutationsAddedAccordingToNetwork,
      mutationsReflectedAccordingToNetwork,
    } = transition;
    let localStore = this.syncQueryManager.snapshot.store.cloneWithoutWrites();
    localStore.ingest(Array.from(pagesChanged.values()));
    const newMutationsUnreflectedAccordingToNetwork: MutationInfo[] = [];
    for (const mutationInfo of this.mutationsUnreflectedAccordingToNetwork) {
      if (
        mutationsReflectedAccordingToNetwork.includes(mutationInfo.mutationId)
      ) {
        continue;
      } else {
        newMutationsUnreflectedAccordingToNetwork.push(mutationInfo);
        const result = executeSyncMutation(
          this.schema,
          this.mutationMap,
          mutationInfo,
          localStore,
        );
        localStore = result.localStore;
      }
    }
    for (const mutationInfo of mutationsAddedAccordingToNetwork) {
      newMutationsUnreflectedAccordingToNetwork.push(mutationInfo);
      const result = executeSyncMutation(
        this.schema,
        this.mutationMap,
        mutationInfo,
        localStore,
      );
      localStore = result.localStore;
    }
    this.mutationsUnreflectedAccordingToNetwork =
      newMutationsUnreflectedAccordingToNetwork;
    const syncQueryManager = this.syncQueryManager.advance({
      // TODO: get rid of this
      localStoreVersion: new LocalStoreVersion(0, 0 as ServerStoreVersion),
      store: localStore,
    });
    const syncQueriesToExecute = Array.from(
      syncQueryManager.getSyncQueriesToExecute(),
    );
    const { responses } = this.executeSyncQueries(
      syncQueriesToExecute,
      syncQueryManager,
    );
    const updates = syncQueryManager.diffResults(this.syncQueryManager);
    this.syncQueryManager = syncQueryManager;
    return [
      ...responses,
      {
        recipient: "UI",
        kind: "transition",
        syncQueryUpdates: updates,
        mutationsApplied: new Set(
          this.mutationsUnreflectedAccordingToNetwork.map(
            (mutationInfo) => mutationInfo.mutationId,
          ),
        ),
      },
    ];
  }

  private handleAddSyncQuerySubscription(
    syncQuerySubscriptionId: SyncQuerySubscriptionId,
    syncQueryFn: SyncQuery,
    syncQueryArgs: DefaultFunctionArgs,
  ): CoreResponse[] {
    const outputs: CoreResponse[] = [];
    this.syncQueries.set(syncQuerySubscriptionId, {
      fn: syncQueryFn,
      args: syncQueryArgs,
    });
    this.syncQueryManager.addSyncQuerySubscription(syncQuerySubscriptionId);
    const { responses, syncQueryUpdates } = this.executeSyncQueries(
      [syncQuerySubscriptionId],
      this.syncQueryManager,
    );
    outputs.push(...responses);
    outputs.push({
      recipient: "UI",
      kind: "transition",
      syncQueryUpdates,
      mutationsApplied: new Set(
        this.mutationsUnreflectedAccordingToNetwork.map(
          (mutationInfo) => mutationInfo.mutationId,
        ),
      ),
    });
    return outputs;
  }

  private handleUnsubscribeFromSyncQuery(
    syncQuerySubscriptionId: SyncQuerySubscriptionId,
  ): CoreResponse[] {
    this.syncQueryManager.clearSyncQuerySubscription(syncQuerySubscriptionId);
    return [];
  }

  private canUnsubscribeFromPageQueries() {
    // We haven't finished running new subscriptions against the
    // currently computed snapshot, so don't unsubscribe in case we need
    // the loaded data
    if (!this.syncQueryManager.haveAllExecuted()) {
      return false;
    }
    // We're running optimistic updates, which might depend on the current
    // data, so don't unsubscribe
    if (this.mutationsUnreflectedAccordingToNetwork.length !== 0) {
      return false;
    }
    // One of our sync queries is still loading, so don't unsubscribe
    // to avoid dropping data we'll need on the next recomputation
    if (!this.syncQueryManager.areAllLoaded()) {
      return false;
    }
    return true;
  }

  private unsubscribeFromPageQueries(): CoreResponse[] {
    if (!this.canUnsubscribeFromPageQueries()) {
      return [];
    }
    const allPageQueries = this.syncQueryManager.getAllPageQueries();
    const unusedPageQueries: Set<ConvexSubscriptionId> = new Set();
    for (const page of this.subscriptions) {
      if (!allPageQueries.has(page)) {
        unusedPageQueries.add(page);
      }
    }
    if (unusedPageQueries.size === 0) {
      return [];
    }
    return [
      {
        recipient: "Network",
        kind: "removeQueryFromNetwork",
        queriesToRemove: Array.from(unusedPageQueries),
      },
    ];
  }

  private executeSyncQueries(
    syncQueryIds: SyncQuerySubscriptionId[],
    syncQueryManager: SyncQueryManager,
  ): {
    responses: CoreResponse[];
    syncQueryUpdates: Map<SyncQuerySubscriptionId, SyncQueryResult>;
  } {
    const allResponses: CoreResponse[] = [];
    const syncQueryUpdates: Map<SyncQuerySubscriptionId, SyncQueryResult> =
      new Map();
    for (const syncQueryId of syncQueryIds) {
      const syncQueryInfo = this.syncQueries.get(syncQueryId);
      if (syncQueryInfo === undefined) {
        throw new Error(`Sync query ${syncQueryId} not found`);
      }
      const { fn, args } = syncQueryInfo;
      const { responses, pagesRead, result, newPage } = executeSyncQuery(
        this.schema,
        fn,
        args,
        syncQueryManager.snapshot.store,
      );
      if (newPage !== null) {
        syncQueryManager.addLoadingPage(newPage.subscriptionId, newPage.args);
      }
      for (const pageId of pagesRead) {
        syncQueryManager.ensureSyncQuerySubscribedToPage(syncQueryId, pageId);
      }

      syncQueryManager.recordSyncQueryResult(syncQueryId, result);
      syncQueryUpdates.set(syncQueryId, result);
      allResponses.push(...responses);
    }
    return { responses: allResponses, syncQueryUpdates };
  }
}
