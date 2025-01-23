import { CoreSyncEngine } from "./core/core";
import {
  CoreRequest,
  CoreResponse,
  UINewSyncQuery,
  UITransition,
} from "./core/protocol";
import { LocalPersistence } from "./localPersistence";
import { Logger } from "./logger";
import { Network } from "./network";

export class Driver {
  coreLocalStore: CoreSyncEngine;
  network: Network;
  localPersistence: LocalPersistence;
  uiTransitionHandler: ((transition: UITransition) => void) | null = null;
  newSyncQueryResultHandler: ((result: UINewSyncQuery) => void) | null = null;
  requestQueue: CoreRequest[] = [];
  logger: Logger;
  constructor(opts: {
    coreLocalStore: CoreSyncEngine;
    network: Network;
    localPersistence: LocalPersistence;
    logger: Logger;
  }) {
    this.coreLocalStore = opts.coreLocalStore;
    this.network = opts.network;
    this.localPersistence = opts.localPersistence;
    this.logger = opts.logger;
  }

  step() {
    for (let i = 0; i < 1000; i++) {
      const requests = [...this.requestQueue];
      this.requestQueue = [];
      for (const request of requests) {
        this.logger.debug("Driver.step: processing request", request);
        const responses = this.coreLocalStore.receive(request);
        for (const response of responses) {
          this.processResponse(response);
        }
      }
      if (this.requestQueue.length === 0) {
        return;
      }
      this.logger.warn(
        "Driver.step: Processing responses added more requests.",
      );
    }
    throw new Error("Too many steps on a single Driver turn!");
  }

  receive(message: CoreRequest) {
    this.requestQueue.push(message);
    this.step();
  }

  addUiTransitionHandler(handler: (transition: UITransition) => void) {
    this.uiTransitionHandler = handler;
  }

  addNewSyncQueryResultHandler(handler: (result: UINewSyncQuery) => void) {
    this.newSyncQueryResultHandler = handler;
  }

  private processResponse(response: CoreResponse) {
    this.logger.debug("Driver.processResponse: processing", response);
    switch (response.kind) {
      case "newSyncQuery": {
        this.newSyncQueryResultHandler?.(response);
        return;
      }
      case "sendQueryToNetwork": {
        this.network.sendQueryToNetwork(
          response.syncFunction,
          response.pageRequest,
        );
        return;
      }
      case "persistMutation": {
        this.localPersistence.persistMutation(
          response.persistId,
          response.mutationInfo,
        );
        return;
      }
      case "persistPages": {
        this.localPersistence.persistPages(response.persistId, response.pages);
        return;
      }
      case "transition": {
        if (!this.uiTransitionHandler) {
          throw new Error("No UI transition handler");
        }
        this.uiTransitionHandler(response);
        return;
      }
      case "removeQueryFromNetwork":
        this.network.removeQueryFromNetwork(response.queriesToRemove);
        return;
      case "sendMutationToNetwork":
        this.network.sendMutationToNetwork(response.mutationInfo);
        return;
    }
    const _typecheck: never = response;
    throw new Error("Unreachable");
  }
}
