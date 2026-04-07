import { api } from "../convex/_generated/api";
import { LocalStoreClient } from "local-store/browser/ui";

import { convexToJson, jsonToConvex } from "convex/values";
import { client } from "./websocket";
import {
  MutationId,
  SyncQueryResult,
  SyncQuerySubscriptionId,
} from "local-store/shared/types";
import { sync } from "../convex/sync/schema";
import { Id } from "../convex/_generated/dataModel";
import { MutationMap } from "local-store/browser/core/optimisticUpdateExecutor";
import { parseMutationInfoJson } from "./protocol";
import { CoreSyncEngine } from "local-store/browser/core/core";
import { Driver } from "local-store/browser/driver";
import { Logger } from "local-store/browser/logger";
import { NetworkImpl } from "local-store/browser/network";
import { NoopLocalPersistence } from "local-store/browser/localPersistence";
import { LocalDbWriter } from "local-store/react/localDb";
import { DataModelFromSchemaDefinition } from "convex/server";
import { LocalDbReader } from "local-store/react/localDb";
import { sync as syncSchema } from "../convex/sync/schema";

type SyncDataModel = DataModelFromSchemaDefinition<typeof syncSchema>;
type QueryCtx = { localDb: LocalDbReader<SyncDataModel> };
type MutationCtx = { localDb: LocalDbWriter<SyncDataModel> };

const allocatedIdsToSyncQuerySubscriptionId = new Map<
  string,
  SyncQuerySubscriptionId
>();
const allocatedIdsToMutationId = new Map<string, MutationId>();

const queries = {
  getConversations: (ctx: QueryCtx, _args: Record<string, never>) => {
    return ctx.localDb
      .query("conversations")
      .withIndex("by_priority")
      .order("desc")
      .take(100);
  },
  getSingleConversation: (
    ctx: QueryCtx,
    args: { conversationId: Id<"conversations"> },
  ) => {
    return ctx.localDb.get("conversations", args.conversationId);
  },
  getUsers: (ctx: QueryCtx, args: { users: Id<"users">[] }) => {
    return args.users.map((id) => ctx.localDb.get("users", id));
  },
  getMessages: (
    ctx: QueryCtx,
    args: { conversationId: Id<"conversations"> },
  ) => {
    const allMessages = ctx.localDb
      .query("messages")
      .withIndex("by_conversation", (q) =>
        q.eq("conversationId", args.conversationId),
      )
      .order("desc")
      .take(5);
    return allMessages.map((m: any) => {
      const user = ctx.localDb.get("users", m.author);
      return {
        ...m,
        author: user?.name ?? "Unknown",
      };
    });
  },
};

const mutations: MutationMap = {
  test: {
    fn: api.messages.send,
    optimisticUpdate: (ctx: MutationCtx, args: any) => {
      ctx.localDb.insert("messages", args.id as Id<"messages">, {
        _creationTime: args.creationTime,
        author: args.author,
        body: args.body,
        conversationId: args.conversationId,
        color: "red",
      });
    },
  },
  "conversations:create": {
    fn: api.conversations.create,
    optimisticUpdate: (ctx: MutationCtx, args: any) => {
      console.log("#### optimisticUpdate conversations:create", args);
      ctx.localDb.insert("conversations", args.id as Id<"conversations">, args);
    },
  },
};

const logger = new Logger();
logger.setLevel("debug");
const coreLocalStore = new CoreSyncEngine(sync, mutations, logger);
const driver = new Driver({
  coreLocalStore,
  network: new NetworkImpl({ convexClient: client }),
  localPersistence: new NoopLocalPersistence(),
  logger,
});

const localStoreClient = new LocalStoreClient({
  driver,
  syncSchema: sync,
  mutations,
});

const currentResults = new Map<SyncQuerySubscriptionId, SyncQueryResult>();
const mutationStatuses = new Map<
  MutationId,
  { status: "unresolved" } | { status: "resolved"; value: any }
>();
export function addSyncQuery(args: {
  id: string;
  name: string;
  udfArgsJson: string;
}): SyncQuerySubscriptionId {
  const query = queries[args.name];
  if (!query) {
    throw new Error(`Unknown sync query: ${args.name}`);
  }
  const udfArgs = jsonToConvex(JSON.parse(args.udfArgsJson)) as any;
  const syncQuerySubscriptionId = localStoreClient.addSyncQuery(
    query,
    udfArgs,
    (result, syncQuerySubscriptionId) => {
      console.debug("syncQuery result", syncQuerySubscriptionId, result);
      currentResults.set(syncQuerySubscriptionId, result);
    },
  );
  allocatedIdsToSyncQuerySubscriptionId.set(args.id, syncQuerySubscriptionId);
  return syncQuerySubscriptionId;
}

export function syncQueryResult(id: string) {
  console.debug("syncQueryResult", id);
  const syncQuerySubscriptionId = allocatedIdsToSyncQuerySubscriptionId.get(id);
  if (!syncQuerySubscriptionId) {
    throw new Error(`Unknown sync query: ${id}`);
  }
  const result = currentResults.get(syncQuerySubscriptionId);
  if (result === undefined) {
    return null;
  }
  if (result.kind === "loading") {
    return { type: "loading" };
  } else if (result.status === "error") {
    return { type: "error", error: result.error.toString() };
  } else {
    return { type: "success", value: convexToJson(result.value) };
  }
}

export function removeSyncQuery(id: string) {
  const syncQuerySubscriptionId = allocatedIdsToSyncQuerySubscriptionId.get(id);
  if (!syncQuerySubscriptionId) {
    throw new Error(`Unknown sync query: ${id}`);
  }
  localStoreClient.removeSyncQuery(syncQuerySubscriptionId);
  currentResults.delete(syncQuerySubscriptionId);
  allocatedIdsToSyncQuerySubscriptionId.delete(id);
  return null;
}

export function requestSyncMutation(args: {
  id: string;
  mutationInfoJson: string;
}) {
  const mutationInfo = parseMutationInfoJson(JSON.parse(args.mutationInfoJson));
  const { mutationPromise, mutationId } = localStoreClient.mutationInternal(
    mutationInfo.mutationPath,
    mutationInfo.optUpdateArgs,
    mutationInfo.serverArgs,
  );
  allocatedIdsToMutationId.set(args.id, mutationId);
  mutationStatuses.set(mutationId, { status: "unresolved" });
  mutationPromise.then((value) => {
    mutationStatuses.set(mutationId, {
      status: "resolved",
      value: convexToJson(value as any),
    });
  });
  return mutationId;
}

export function getSyncMutationStatus(
  id: string,
):
  | { status: "reflected" }
  | { status: "reflectedLocallyButWaitingForNetwork" }
  | { status: "unresolved" }
  | { status: "reflectedOnNetworkButNotLocally" } {
  const mutationId = allocatedIdsToMutationId.get(id);
  if (!mutationId) {
    throw new Error(`Unknown mutation: ${id}`);
  }
  const status = localStoreClient.getMutationStatus(mutationId);
  if (!status) {
    throw new Error(`Unknown mutation: ${mutationId}`);
  }
  return { status: status.status };
}
