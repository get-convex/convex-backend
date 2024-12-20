import { Channel } from "async-channel";
import {
  ClientId,
  followerMessage,
  FollowerMessage,
  leaderMessage,
  LeaderMessage,
  mutationToStoredMutation,
  pageToStoredPage,
  storedPageToPage,
} from "./types";
import Dexie from "dexie";
import { LocalPersistence } from "../localPersistence";
import { CorePersistenceRequest, Page } from "../core/protocol";
import { MutationInfo, PersistId } from "../../shared/types";

const DB_VERSION = 1;

type WorkerMessage =
  | { type: "follower"; message: FollowerMessage }
  | { type: "leader"; message: LeaderMessage };

export type ElectionOptions = {
  joinTimeoutMs?: number;
  debug?: boolean;
};

/**
 * Leader election for IndexedDB persistence.
 *
 * This class relies on the Lock APIs to ensure that only one
 * instance of it across an origin is elected the *leader*. LocalPersistence calls
 * then send messages to the leader, which performs the desired action and retries.
 *
 * Each instance of the class has a "worker" thread that receives messages, connects
 * to the leader if it can, and otherwise decides to become the leader.
 */
export class Election implements LocalPersistence {
  clientId: ClientId;

  private currentState: State;

  /**
   * Broadcast channels for communicating with other threads. Both callers into
   * this class and the worker send follower messages, and the single leader
   * across all threads sends leader messages.
   */
  followerBroadcast: BroadcastChannel;
  leaderBroadcast: BroadcastChannel;

  /**
   * Channel for communicating with the worker thread. Both broadcast channels
   * push messages onto this channel.
   */
  workerChannel: Channel<WorkerMessage>;

  joinTimeoutMs: number;
  debug: boolean;

  constructor(
    private name: string,
    private address: string,
    options?: ElectionOptions,
  ) {
    this.joinTimeoutMs = options?.joinTimeoutMs ?? 1000;
    this.debug = options?.debug ?? false;
    this.clientId = crypto.randomUUID();

    this.currentState = new State();

    this.workerChannel = new Channel(16);
    this.followerBroadcast = new BroadcastChannel(`${this.name}-client`);
    this.followerBroadcast.onmessage = (e) => {
      this.log(`Received follower message`, e.data);
      const message = followerMessage.parse(e.data);
      this.workerChannel.push({ type: "follower", message });
    };
    this.followerBroadcast.onmessageerror = (e) => {
      this.log("Client broadcast message error:", e);
    };
    this.leaderBroadcast = new BroadcastChannel(`${this.name}-server`);
    this.leaderBroadcast.onmessage = (e) => {
      this.log("Received leader message", e.data);
      const message = leaderMessage.parse(e.data);
      this.workerChannel.push({ type: "leader", message });
    };
    this.leaderBroadcast.onmessageerror = (e) => {
      this.log("Server broadcast message error:", e);
    };

    void this.go();
  }

  addListener(listener: (request: CorePersistenceRequest) => void) {
    this.currentState.addListener(listener);
  }

  persistMutation(persistId: PersistId, mutationInfo: MutationInfo) {
    this.broadcastFollowerMessage({
      type: "persist",
      clientId: this.clientId,
      persistId,
      pages: [],
      mutationInfos: [mutationToStoredMutation(mutationInfo)],
    });
  }

  persistPages(persistId: PersistId, pages: Page[]) {
    const storedPages = [];
    for (const page of pages) {
      const storedPage = pageToStoredPage(page);
      if (storedPage !== null) {
        storedPages.push(storedPage);
      }
    }
    this.broadcastFollowerMessage({
      type: "persist",
      clientId: this.clientId,
      persistId,
      pages: storedPages,
      mutationInfos: [],
    });
  }

  destroy() {
    this.followerBroadcast.close();
    this.leaderBroadcast.close();
    this.workerChannel.close();
  }

  private log(message: string, ..._args: any[]) {
    if (this.debug) {
      console.log(`[election] ${message}`);
    }
  }

  private async leaderExists(): Promise<boolean> {
    const locks = await navigator.locks.query();
    return locks.held?.find((l) => l.name === this.name) !== undefined;
  }

  private broadcastFollowerMessage(message: FollowerMessage) {
    this.log("Broadcasting follower message", message);
    this.followerBroadcast.postMessage(message);

    // NB: BroadcastChannels do not send messages to themselves,
    // so we push this message onto the worker channel in case we're the leader.
    this.workerChannel.push({ type: "follower", message });
  }

  private broadcastLeaderMessage(message: LeaderMessage) {
    this.log("Broadcasting leader message", message);
    this.leaderBroadcast.postMessage(message);
    this.workerChannel.push({ type: "leader", message });
  }

  private async go() {
    while (!this.workerChannel.done) {
      const tryLeadership = await this.follow();
      if (tryLeadership) {
        await navigator.locks.request(
          this.name,
          { mode: "exclusive", ifAvailable: true },
          (lock) => this.lead(lock),
        );
      }
    }
  }

  // Cleanly exits after the leader goes away, returning whether if we should
  // try to become the leader.
  private async follow(): Promise<boolean> {
    const leaderExists = await this.leaderExists();
    if (!leaderExists) {
      return true;
    }
    // Try to join the current leader.
    console.log("Trying to join as a follower...");
    this.broadcastFollowerMessage({
      type: "join",
      clientId: this.clientId,
      name: this.name,
      address: this.address,
    });

    let leaderClientId: string | null = null;
    const deadline = Date.now() + this.joinTimeoutMs;
    while (Date.now() < deadline) {
      const workerMessage = await this.workerChannel.get();
      if (workerMessage.type !== "leader") {
        continue;
      }
      if (workerMessage.message.requestingClientId === this.clientId) {
        leaderClientId = this.handleJoinResult(workerMessage.message);
        break;
      }
    }
    if (leaderClientId === null) {
      throw new Error("Leader not found");
    }

    while (!this.workerChannel.done) {
      const leaderExists = await this.leaderExists();
      if (!leaderExists) {
        return true;
      }
      const workerMessage = await this.workerChannel.get();
      if (workerMessage.type === "follower") {
        continue;
      }
      // TODO: Handle retrying a message if a leader crashes
      // while servicing a request or times out.
      const { message } = workerMessage;
      if (message.requestingClientId !== this.clientId) {
        continue;
      }
      this.handleLeaderMessage(message);
    }

    return false;
  }

  private async lead(lock: Lock | null) {
    // If someone else grabbed the lock, return early and try to follow again.
    if (lock === null) {
      return;
    }

    console.log("Starting as leader...");
    const db = new Dexie(this.name);
    db.version(DB_VERSION).stores({
      // TODO: Normalize objects across pages.
      pages: "[table+index+serializedLowerBound]",
    });

    // Subscribe to ourselves if we're now the leader.
    this.broadcastFollowerMessage({
      type: "join",
      clientId: this.clientId,
      name: this.name,
      address: this.address,
    });

    while (!this.workerChannel.done) {
      const workerMessage = await this.workerChannel.get();
      this.log("Received worker message", workerMessage);
      if (workerMessage.type === "leader") {
        const { message } = workerMessage;
        if (message.leaderClientId !== this.clientId) {
          throw new Error(
            `Someone else sending a leader message while we have the lock?`,
          );
        }
        if (message.requestingClientId === this.clientId) {
          this.handleLeaderMessage(message);
        }
        continue;
      }
      const { message } = workerMessage;
      switch (message.type) {
        case "join": {
          const storedPages = await db.table("pages").toArray();
          this.broadcastLeaderMessage({
            type: "joinResult",
            leaderClientId: this.clientId,
            requestingClientId: message.clientId,
            result: {
              type: "success",
              pages: storedPages,
              mutations: [],
            },
          });
          break;
        }
        case "persist": {
          await db.transaction("rw", "pages", async () => {
            await db.table("pages").bulkPut(message.pages);
          });
          this.broadcastLeaderMessage({
            type: "persistResult",
            leaderClientId: this.clientId,
            requestingClientId: message.clientId,
            persistId: message.persistId,
            result: {
              type: "success",
            },
          });
          break;
        }
        default: {
          throw new Error(`Invalid message: ${JSON.stringify(message)}`);
        }
      }
    }
  }

  private handleLeaderMessage(message: LeaderMessage) {
    switch (message.type) {
      case "joinResult": {
        this.handleJoinResult(message);
        break;
      }
      case "persistResult": {
        this.handlePersistResult(message);
        break;
      }
      default: {
        throw new Error(`Unexpected message ${message}.`);
      }
    }
  }

  private handleJoinResult(message: LeaderMessage) {
    if (message.type !== "joinResult") {
      throw new Error(`Unexpected message type ${message.type}.`);
    }
    if (message.result.type === "failure") {
      throw new Error(`Can't join leader: ${message.result.error}`);
    }
    console.log(`Connected to ${message.leaderClientId}!`);

    const { pages: storedPages, mutations: _storedMutations } = message.result;
    const pages = storedPages.map(storedPageToPage);

    // TODO: Handle reloading mutations.
    this.currentState.setCurrentPages(pages);
    return message.leaderClientId;
  }

  private handlePersistResult(message: LeaderMessage) {
    if (message.type !== "persistResult") {
      throw new Error(`Unexpected message type ${message.type}.`);
    }
    this.currentState.emitToListeners({
      requestor: "LocalPersistence",
      kind: "localPersistComplete",
      persistId: message.persistId as PersistId,
    });
  }
}

class State {
  listeners: Set<(request: CorePersistenceRequest) => void> = new Set();
  currentPages?: Page[];

  addListener(listener: (request: CorePersistenceRequest) => void) {
    this.listeners.add(listener);
    if (this.currentPages) {
      listener({
        requestor: "LocalPersistence",
        kind: "ingestFromLocalPersistence",
        pages: this.currentPages,
        serverTs: 0,
      });
    }
  }

  setCurrentPages(pages: Page[]) {
    // TODO: Handle reloading from persistence when we switch from
    // one leader to another.
    const firstTime = this.currentPages === undefined;
    this.currentPages = pages;
    if (firstTime) {
      this.emitToListeners({
        requestor: "LocalPersistence",
        kind: "ingestFromLocalPersistence",
        pages,
        serverTs: 0,
      });
    }
  }

  emitToListeners(request: CorePersistenceRequest) {
    for (const listener of this.listeners) {
      listener(request);
    }
  }
}
