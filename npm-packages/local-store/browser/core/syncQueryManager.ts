import {
  ConvexSubscriptionId,
  LocalStoreVersion,
  PageArguments,
  SyncQueryResult,
  SyncQuerySubscriptionId,
} from "../../shared/types";
import { CopyOnWriteLocalStore } from "./localStore";

type Snapshot = {
  store: CopyOnWriteLocalStore;
  localStoreVersion: LocalStoreVersion;
};

export class SyncQueryManager {
  private pageQueryToSyncQueries: Map<
    ConvexSubscriptionId,
    Set<SyncQuerySubscriptionId>
  > = new Map();
  private syncQueryToPageQuery: Map<
    SyncQuerySubscriptionId,
    Set<ConvexSubscriptionId>
  > = new Map();

  private _snapshot: Snapshot;

  private syncQueryToResult: Map<
    SyncQuerySubscriptionId,
    | {
        kind: "NotStarted";
      }
    | { kind: "Ready"; result: SyncQueryResult }
  > = new Map();

  constructor(snapshot: Snapshot) {
    this._snapshot = snapshot;
  }

  get localStoreVersion(): LocalStoreVersion {
    return this.snapshot.localStoreVersion;
  }

  get snapshot(): Snapshot {
    return this._snapshot;
  }

  addSyncQuerySubscription(syncQuerySubscriptionId: SyncQuerySubscriptionId) {
    if (this.syncQueryToPageQuery.has(syncQuerySubscriptionId)) {
      throw new Error("Sync query already subscribed");
    }
    this.syncQueryToPageQuery.set(
      syncQuerySubscriptionId,
      new Set<ConvexSubscriptionId>(),
    );
    this.syncQueryToResult.set(syncQuerySubscriptionId, { kind: "NotStarted" });
  }

  ensureSyncQuerySubscribedToPage(
    syncQuerySubscriptionId: SyncQuerySubscriptionId,
    pageQuery: ConvexSubscriptionId,
  ) {
    const syncQueries = this.pageQueryToSyncQueries.get(pageQuery);
    if (!syncQueries) {
      this.pageQueryToSyncQueries.set(
        pageQuery,
        new Set([syncQuerySubscriptionId]),
      );
    } else {
      syncQueries.add(syncQuerySubscriptionId);
    }
    const pageQueries = this.syncQueryToPageQuery.get(syncQuerySubscriptionId);
    if (!pageQueries) {
      this.syncQueryToPageQuery.set(
        syncQuerySubscriptionId,
        new Set([pageQuery]),
      );
    } else {
      pageQueries.add(pageQuery);
    }
  }

  clearSyncQuerySubscription(syncQuerySubscriptionId: SyncQuerySubscriptionId) {
    const pageQueries = this.syncQueryToPageQuery.get(syncQuerySubscriptionId);
    this.syncQueryToPageQuery.delete(syncQuerySubscriptionId);
    if (pageQueries === undefined) {
      return;
    }
    for (const pageQuery of pageQueries) {
      const syncQueries = this.pageQueryToSyncQueries.get(pageQuery);
      if (syncQueries === undefined) {
        continue;
      }
      syncQueries.delete(syncQuerySubscriptionId);
      if (syncQueries.size === 0) {
        this.pageQueryToSyncQueries.delete(pageQuery);
      }
    }
  }

  recordSyncQueryResult(
    syncQuerySubscriptionId: SyncQuerySubscriptionId,
    result: SyncQueryResult,
  ) {
    this.syncQueryToResult.set(syncQuerySubscriptionId, {
      kind: "Ready",
      result,
    });
  }

  advance(nextSnapshot: Snapshot): SyncQueryManager {
    const staleSyncQueries: Set<SyncQuerySubscriptionId> = new Set();
    const updatedPages = this.snapshot.store.getChangedPages(
      nextSnapshot.store,
    );
    for (const updatedPage of updatedPages) {
      for (const syncQueryId of this.pageQueryToSyncQueries.get(updatedPage) ??
        []) {
        staleSyncQueries.add(syncQueryId);
      }
    }
    const newSyncQueryManager = new SyncQueryManager(nextSnapshot);
    newSyncQueryManager.ingest(this);
    for (const syncQueryId of staleSyncQueries) {
      newSyncQueryManager.clearSyncQuerySubscription(syncQueryId);
      newSyncQueryManager.syncQueryToResult.set(syncQueryId, {
        kind: "NotStarted",
      });
    }
    return newSyncQueryManager;
  }

  getAllPageQueries(): Set<ConvexSubscriptionId> {
    return new Set(this.pageQueryToSyncQueries.keys());
  }

  diffResults(
    other: SyncQueryManager,
  ): Map<SyncQuerySubscriptionId, SyncQueryResult> {
    const diff = new Map<SyncQuerySubscriptionId, SyncQueryResult>();
    for (const [syncQueryId, result] of this.syncQueryToResult) {
      if (other.syncQueryToResult.get(syncQueryId) !== result) {
        diff.set(
          syncQueryId,
          result.kind === "Ready" ? result.result : { kind: "loading" },
        );
      }
    }
    return diff;
  }

  addLoadingPage(
    convexSubscriptionId: ConvexSubscriptionId,
    pageArguments: PageArguments,
  ) {
    this._snapshot.store.ingest([
      {
        tableName: pageArguments.syncTableName,
        indexName: pageArguments.index,
        convexSubscriptionId,
        state: { kind: "loading", target: pageArguments.target },
      },
    ]);
  }

  ingest(other: SyncQueryManager) {
    for (const [syncQueryId, pageQueries] of other.syncQueryToPageQuery) {
      this.addSyncQuerySubscription(syncQueryId);
      for (const pageQuery of pageQueries) {
        this.ensureSyncQuerySubscribedToPage(syncQueryId, pageQuery);
      }
    }
    for (const [syncQueryId, result] of other.syncQueryToResult) {
      this.syncQueryToResult.set(syncQueryId, result);
    }
  }

  haveAllExecuted(): boolean {
    for (const result of this.syncQueryToResult.values()) {
      if (result.kind !== "Ready") {
        return false;
      }
    }
    return true;
  }

  areAllLoaded(): boolean {
    for (const result of this.syncQueryToResult.values()) {
      if (result.kind === "NotStarted") {
        return true;
      }
      if (result.kind === "Ready" && result.result.kind === "loading") {
        return true;
      }
    }
    return false;
  }

  getSyncQueriesToExecute(): Set<SyncQuerySubscriptionId> {
    const syncQueriesToExecute: Set<SyncQuerySubscriptionId> = new Set();
    for (const [syncQueryId, result] of this.syncQueryToResult) {
      if (result.kind !== "Ready") {
        syncQueriesToExecute.add(syncQueryId);
      }
    }
    return syncQueriesToExecute;
  }
}
