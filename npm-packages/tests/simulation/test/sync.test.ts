import { expect, test } from "vitest";
import { Driver } from "local-store/browser/driver";
import { CoreSyncEngine } from "local-store/browser/core/core";
import { sync as syncSchema } from "../convex/sync/schema";
import { MutationMap } from "local-store/browser/core/optimisticUpdateExecutor";
import { Id } from "../convex/_generated/dataModel";
import { api } from "../convex/_generated/api";
import { NoopLocalPersistence } from "local-store/browser/localPersistence";
import {
  MutationId,
  SyncQueryResult,
  SyncQuerySubscriptionId,
} from "local-store/shared/types";
import { LocalDbReader, LocalDbWriter } from "local-store/react/localDb";
import { Logger } from "local-store/browser/logger";
import { LocalStoreClient } from "local-store/browser/ui";
import { JSONValue, jsonToConvex } from "convex/values";
import { NetworkImpl } from "local-store/browser/network";
import { BaseConvexClient } from "convex/browser";
import { DataModelFromSchemaDefinition } from "convex/server";

type SyncDataModel = DataModelFromSchemaDefinition<typeof syncSchema>;
type QueryCtx = { localDb: LocalDbReader<SyncDataModel> };
type MutationCtx = { localDb: LocalDbWriter<SyncDataModel> };

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
      ctx.localDb.insert("conversations", args.id as Id<"conversations">, args);
    },
  },
};

class TestingWebSocket {
  onopen?: (this: TestingWebSocket, ev: Event) => any;
  onerror?: (this: TestingWebSocket, ev: Event) => any;
  onmessage?: (this: TestingWebSocket, ev: MessageEvent) => any;
  onclose?: (this: TestingWebSocket, ev: CloseEvent) => any;

  constructor(_url: string | URL, _protocols?: string | string[]) {
    // Call this in a `setTimeout` since the `onopen` callback gets added
    // just after the WS is constructed
    setTimeout(() => {
      console.log("WebSocket connected", this.onopen);
      this.onopen?.(new Event("open"));
    }, 0);
  }

  send(data: string | ArrayBuffer | Blob | ArrayBufferView) {
    if (typeof data !== "string") {
      throw new Error("Only strings are supported");
    }
    // Drop any messages sent by the client on the floor, and assume
    // that the list of server messages we have is correct given the
    // client messages we send
  }

  ingestServerMessage(message: any) {
    this.onmessage?.({ data: message } as any);
  }

  close() {
    console.log("WebSocket closed");
    this.onclose?.({ code: 1000 } as any);
  }
}

function getSocket(client: BaseConvexClient) {
  return (client as any).webSocketManager.socket;
}

async function waitUntilWsIsReady(
  client: BaseConvexClient,
): Promise<TestingWebSocket> {
  let attempts = 0;
  let ws = getSocket(client);
  while (ws.state !== "ready" && attempts < 5) {
    console.log("WebSocket state:", ws);
    await new Promise((resolve) => setTimeout(resolve, 1000));
    attempts++;
    ws = getSocket(client);
  }
  if (ws.state !== "ready") {
    throw new Error("WebSocket is not ready");
  }
  return ws.ws;
}

async function sendWsMessage(client: BaseConvexClient, message: any) {
  const ws = await waitUntilWsIsReady(client);
  ws.ingestServerMessage(message);
}

async function init(): Promise<{
  driver: Driver;
  uiClient: LocalStoreClient;
  convexClient: BaseConvexClient;
}> {
  const address = "https://suadero.example.com";
  const convexClient = new BaseConvexClient(address, () => {}, {
    unsavedChangesWarning: false,
    skipConvexDeploymentUrlCheck: true,
    webSocketConstructor: TestingWebSocket as any,
    verbose: true,
  });
  const network = new NetworkImpl({
    convexClient,
  });
  const logger = new Logger();
  logger.setLevel("debug");
  const driver = new Driver({
    coreLocalStore: new CoreSyncEngine(syncSchema, mutations, logger),
    network,
    localPersistence: new NoopLocalPersistence(),
    logger,
  });
  const uiClient = new LocalStoreClient({
    syncSchema,
    mutations,
    driver,
  });
  await waitUntilWsIsReady(convexClient);

  return { driver, uiClient, convexClient };
}

type Message =
  | {
      kind: "addSyncQuerySubscription";
      id: string;
      syncQueryFn: string;
      syncQueryArgs: JSONValue;
    }
  | {
      kind: "removeSyncQuerySubscription";
      id: string;
    }
  | {
      kind: "mutate";
      id: string;
      mutationFn: string;
      optUpdateArgs: JSONValue;
      serverArgs: JSONValue;
    }
  | {
      kind: "wsMessage";
      message: string;
    }
  | {
      kind: "checkSyncQueryResult";
      id: string;
      expectedResult: SyncQueryResult;
    }
  | {
      kind: "checkMutationStatus";
      id: string;
      expectedStatus: any;
    }
  | {
      kind: "waitForAllMessagesProcessed";
    };

async function runTest(
  {
    uiClient,
    convexClient,
  }: { uiClient: LocalStoreClient; convexClient: BaseConvexClient },
  messages: Array<Message>,
) {
  const allocatedIdsToSyncQuerySubscriptionId = new Map<
    string,
    SyncQuerySubscriptionId
  >();
  const allocatedIdsToMutationId = new Map<string, MutationId>();
  const queryResults = new Map<string, SyncQueryResult>();

  for (const message of messages) {
    console.log("#### begin message", message.kind);
    switch (message.kind) {
      case "addSyncQuerySubscription": {
        const syncQueryFn = queries[message.syncQueryFn];
        const syncQuerySubscriptionId = uiClient.addSyncQuery(
          syncQueryFn,
          jsonToConvex(message.syncQueryArgs) as any,
          (result) => {
            console.log(
              "addSyncQuerySubscription on update",
              message.id,
              result,
            );
            queryResults.set(message.id, result);
          },
        );
        allocatedIdsToSyncQuerySubscriptionId.set(
          message.id,
          syncQuerySubscriptionId,
        );
        if (!queryResults.has(message.id)) {
          queryResults.set(message.id, {
            kind: "loading",
          });
        }
        break;
      }
      case "removeSyncQuerySubscription": {
        const syncQuerySubscriptionId =
          allocatedIdsToSyncQuerySubscriptionId.get(message.id);
        if (!syncQuerySubscriptionId) {
          throw new Error(
            `Sync query subscription id not found for id: ${message.id}`,
          );
        }
        allocatedIdsToSyncQuerySubscriptionId.delete(message.id);
        uiClient.removeSyncQuery(syncQuerySubscriptionId);
        break;
      }
      case "mutate": {
        const { mutationId } = uiClient.mutationInternal(
          message.mutationFn as any,
          jsonToConvex(message.optUpdateArgs) as any,
          jsonToConvex(message.serverArgs) as any,
        );
        allocatedIdsToMutationId.set(message.id, mutationId);
        break;
      }
      case "wsMessage": {
        await sendWsMessage(convexClient, message.message);
        break;
      }
      case "checkSyncQueryResult": {
        console.log("checkSyncQueryResult", queryResults, message.id);
        const result = queryResults.get(message.id);
        console.log("result", result);
        console.log("expectedResult", message.expectedResult);
        expect(result).toMatchObject(message.expectedResult);
        break;
      }
      case "checkMutationStatus": {
        const mutationId = allocatedIdsToMutationId.get(message.id);
        if (!mutationId) {
          throw new Error(`Mutation id not found for id: ${message.id}`);
        }
        const result = uiClient.getMutationStatus(mutationId);
        expect(result).toEqual(message.expectedStatus);
        break;
      }
      case "waitForAllMessagesProcessed": {
        await uiClient.waitForTransitionToComplete();
        // This is a hack, but deals with things that happen on the next tick, like calling `onUpdate`
        await new Promise((resolve) => setTimeout(resolve, 0));
        break;
      }
    }
    console.log("#### end message", message.kind);
  }
}

test("sync", async () => {
  const { uiClient, convexClient } = await init();
  const messages: Array<Message> = [
    {
      kind: "wsMessage",
      message:
        '{"type":"Transition","startVersion":{"querySet":0,"identity":0,"ts":"AAAAAAAAAAA="},"endVersion":{"querySet":1,"identity":0,"ts":"D2DML38ZfBY="},"modifications":[]}',
    },
    {
      kind: "waitForAllMessagesProcessed",
    },
    {
      kind: "addSyncQuerySubscription",
      id: "556511cb-a5b1-4314-aa7f-69f9146792c5",
      syncQueryFn: "getConversations",
      syncQueryArgs: {},
    },
    {
      kind: "waitForAllMessagesProcessed",
    },
    {
      kind: "checkSyncQueryResult",
      id: "556511cb-a5b1-4314-aa7f-69f9146792c5",
      expectedResult: {
        kind: "loading",
      },
    },
    {
      kind: "waitForAllMessagesProcessed",
    },
    {
      kind: "wsMessage",
      message:
        '{"type":"Transition","startVersion":{"querySet":1,"identity":0,"ts":"D2DML38ZfBY="},"endVersion":{"querySet":2,"identity":0,"ts":"D2DML38ZfBY="},"modifications":[{"type":"QueryUpdated","queryId":0,"value":{"lowerBound":{"kind":"predecessor","value":[]},"results":[],"upperBound":{"kind":"successor","value":[]}},"logLines":[],"journal":null}]}',
    },
    {
      kind: "waitForAllMessagesProcessed",
    },
    {
      kind: "checkSyncQueryResult",
      id: "556511cb-a5b1-4314-aa7f-69f9146792c5",
      expectedResult: {
        kind: "loaded",
        status: "success",
        value: [],
      },
    },
    {
      kind: "mutate",
      id: "7012afe0-de4a-4462-bedf-498c66e825f6",
      mutationFn: "conversations:create",
      optUpdateArgs: { emoji: "a", id: "a" },
      serverArgs: { emoji: "a" },
    },
    {
      kind: "waitForAllMessagesProcessed",
    },
    {
      kind: "checkSyncQueryResult",
      id: "556511cb-a5b1-4314-aa7f-69f9146792c5",
      expectedResult: {
        kind: "loaded",
        status: "success",
        value: [{ emoji: "a", id: "a" }],
      },
    },
    {
      kind: "mutate",
      id: "4675c729-b468-4eb2-b8bf-52cb17f1dfcc",
      mutationFn: "conversations:create",
      optUpdateArgs: { emoji: "b", id: "b" },
      serverArgs: { emoji: "b" },
    },
    {
      kind: "waitForAllMessagesProcessed",
    },
    {
      kind: "checkSyncQueryResult",
      id: "556511cb-a5b1-4314-aa7f-69f9146792c5",
      expectedResult: {
        kind: "loaded",
        status: "success",
        value: [
          { emoji: "b", id: "b" },
          { emoji: "a", id: "a" },
        ],
      },
    },
  ];
  await runTest({ uiClient, convexClient }, messages);
});
