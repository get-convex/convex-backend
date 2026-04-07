import {
  CorePersistenceRequest,
  Page,
} from "local-store/browser/core/protocol";
import { PersistId } from "local-store/shared/types";
import { LocalPersistence } from "local-store/browser/localPersistence";
import { MutationInfo } from "local-store/shared/types";
import { mutationInfoToJson, outgoingMessages, pageToJson } from "./protocol";

export class TestLocalPersistence implements LocalPersistence {
  listeners: Set<(request: CorePersistenceRequest) => void> = new Set();

  addListener(listener: (request: CorePersistenceRequest) => void) {
    this.listeners.add(listener);
    listener({
      requestor: "LocalPersistence",
      kind: "ingestFromLocalPersistence",
      pages: [],
      serverTs: 0,
    });
  }

  emitMessage(message: CorePersistenceRequest) {
    for (const listener of this.listeners) {
      listener(message);
    }
  }

  persistMutation(persistId: PersistId, mutationInfo: MutationInfo) {
    outgoingMessages.push({
      type: "persistMutation",
      persistId,
      mutationInfo: mutationInfoToJson(mutationInfo),
    });
  }

  persistPages(persistId: PersistId, pages: Page[]) {
    outgoingMessages.push({
      type: "persistPages",
      persistId,
      pages: pages.map(pageToJson),
    });
  }
}

export const localPersistence = new TestLocalPersistence();
