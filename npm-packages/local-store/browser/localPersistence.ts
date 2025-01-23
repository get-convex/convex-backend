import { MutationInfo, PersistId } from "../shared/types";
import { CorePersistenceRequest, Page } from "./core/protocol";

export interface LocalPersistence {
  addListener(listener: (request: CorePersistenceRequest) => void): void;
  persistMutation(persistId: PersistId, mutationInfo: MutationInfo): void;
  persistPages(persistId: PersistId, pages: Page[]): void;
}

export class NoopLocalPersistence implements LocalPersistence {
  private listeners: Set<(request: CorePersistenceRequest) => void> = new Set();

  addListener(listener: (request: CorePersistenceRequest) => void) {
    this.listeners.add(listener);
    setTimeout(() => {
      listener({
        requestor: "LocalPersistence",
        kind: "ingestFromLocalPersistence",
        pages: [],
        serverTs: 0,
      });
    }, 0);
  }

  persistMutation(persistId: PersistId, _mutationInfo: MutationInfo) {
    setTimeout(() => {
      for (const listener of this.listeners) {
        listener({
          requestor: "LocalPersistence",
          kind: "localPersistComplete",
          persistId,
        });
      }
    }, 0);
  }

  persistPages(persistId: PersistId, _pages: Page[]) {
    setTimeout(() => {
      for (const listener of this.listeners) {
        listener({
          requestor: "LocalPersistence",
          kind: "localPersistComplete",
          persistId,
        });
      }
    }, 0);
  }
}
