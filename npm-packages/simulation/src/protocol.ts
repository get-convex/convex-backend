import { Key, MutationId } from "local-store/shared/types";
import { LowerBound, PageResult, UpperBound } from "local-store/shared/types";
import { MutationInfo } from "local-store/shared/types";
import { MAXIMAL_KEY, MINIMAL_KEY, PersistId } from "local-store/shared/types";
import { webSockets } from "./websocket";
import { convexToJson, jsonToConvex } from "convex/values";
import { FunctionReference, getFunctionName } from "convex/server";
import { Page } from "local-store/browser/core/protocol";
import { localPersistence } from "./indexedDb";

export type MutationInfoJSON = {
  mutationName: string;
  mutationId: string;
  mutationPath: string;
  // serialized via convexToJson
  optUpdateArgs: any;
  // serialized via convexToJson
  serverArgs: any;
};

export type PageJSON = {
  tableName: string;
  indexName: string;
  convexSubscriptionId: string;
  state:
    | { kind: "loaded"; value: PageResultJSON }
    | { kind: "loading"; target: KeyJSON };
};

export type PageResultJSON = {
  // serialized via convexToJson
  results: any[];
  lowerBound: LowerBoundJSON;
  upperBound: UpperBoundJSON;
};

export type LowerBoundJSON =
  | { kind: "successor"; value: IndexPrefixJSON }
  | typeof MINIMAL_KEY;

// serialized via convexToJson
export type IndexPrefixJSON = any[];

export type UpperBoundJSON = ExactKeyJSON | typeof MAXIMAL_KEY;

export type ExactKeyJSON = {
  kind: "exact";
  value: IndexKeyJSON;
};

// serialized via convexToJson
export type IndexKeyJSON = any[];

export type KeyJSON =
  | { kind: "successor" | "predecessor"; value: IndexPrefixJSON }
  | ExactKeyJSON;

type OutgoingMessage =
  // WebSocket messages
  | { type: "connect"; webSocketId: number }
  | {
      type: "send";
      webSocketId: number;
      data: string;
    }
  | { type: "close"; webSocketId: number }
  // Persistence messages
  | {
      type: "persistMutation";
      persistId: string;
      mutationInfo: MutationInfoJSON;
    }
  | {
      type: "persistPages";
      persistId: string;
      pages: Array<PageJSON>;
    }
  | {
      type: "mutationDone";
      mutationId: number;
      result:
        | { type: "success"; value: any }
        | { type: "failure"; error: string };
    };

export const outgoingMessages: OutgoingMessage[] = [];

export type IncomingMessage =
  // WebSocket messages
  | { type: "connected"; webSocketId: number }
  | { type: "message"; webSocketId: number; data: string }
  | { type: "closed"; webSocketId: number }
  // Persistence messages
  | { type: "persistenceDone"; persistId: string; error?: string };

export function getOutgoingMessages() {
  const result = [...outgoingMessages];
  outgoingMessages.length = 0;
  return result;
}

export function receiveIncomingMessages(messages: IncomingMessage[]) {
  for (const message of messages) {
    switch (message.type) {
      case "connected": {
        const ws = webSockets.get(message.webSocketId);
        if (!ws) {
          throw new Error(`Unknown websocket id: ${message.webSocketId}`);
        }
        if (ws.onopen) {
          ws.onopen(new Event("open"));
        }
        break;
      }
      case "message": {
        const ws = webSockets.get(message.webSocketId);
        if (!ws) {
          throw new Error(`Unknown websocket id: ${message.webSocketId}`);
        }
        if (ws.onmessage) {
          ws.onmessage({ data: message.data } as any);
        }
        break;
      }
      case "closed": {
        const ws = webSockets.get(message.webSocketId);
        if (!ws) {
          throw new Error(`Unknown websocket id: ${message.webSocketId}`);
        }
        if (ws.onclose) {
          ws.onclose({ code: 1000 } as any);
        }
        webSockets.delete(message.webSocketId);
        break;
      }
      case "persistenceDone": {
        localPersistence.emitMessage({
          requestor: "LocalPersistence",
          kind: "localPersistComplete",
          persistId: message.persistId as PersistId,
        });
        break;
      }
      default: {
        const _: never = message;
      }
    }
  }
}

export function mutationInfoToJson(
  mutationInfo: MutationInfo,
): MutationInfoJSON {
  return {
    mutationName: mutationInfo.mutationName,
    mutationId: mutationInfo.mutationId,
    mutationPath: getFunctionName(mutationInfo.mutationPath),
    optUpdateArgs: convexToJson(mutationInfo.optUpdateArgs as any),
    serverArgs: convexToJson(mutationInfo.serverArgs as any),
  };
}

export function parseMutationInfoJson(
  mutationInfoJson: MutationInfoJSON,
): MutationInfo {
  return {
    mutationName: mutationInfoJson.mutationName,
    mutationId: mutationInfoJson.mutationId as MutationId,
    mutationPath:
      mutationInfoJson.mutationPath as unknown as FunctionReference<"mutation">,
    optUpdateArgs: jsonToConvex(mutationInfoJson.optUpdateArgs) as any,
    serverArgs: jsonToConvex(mutationInfoJson.serverArgs) as any,
  };
}

export function pageToJson(page: Page): PageJSON {
  let state: PageJSON["state"];
  if (page.state.kind === "loaded") {
    state = { kind: "loaded", value: pageResultToJson(page.state.value) };
  } else {
    state = { kind: "loading", target: keyToJson(page.state.target) };
  }
  return {
    tableName: page.tableName,
    indexName: page.indexName,
    convexSubscriptionId: page.convexSubscriptionId,
    state,
  };
}

function pageResultToJson(pageResult: PageResult): PageResultJSON {
  return {
    results: pageResult.results,
    lowerBound: lowerBoundToJson(pageResult.lowerBound),
    upperBound: upperBoundToJson(pageResult.upperBound),
  };
}

function lowerBoundToJson(lowerBound: LowerBound): LowerBoundJSON {
  return {
    kind: lowerBound.kind,
    value: convexToJson(lowerBound.value as any) as any,
  };
}

function upperBoundToJson(upperBound: UpperBound): UpperBoundJSON {
  return {
    kind: upperBound.kind,
    value: convexToJson(upperBound.value as any) as any,
  };
}

function keyToJson(key: Key): KeyJSON {
  return {
    kind: key.kind,
    value: convexToJson(key.value as any) as any,
  };
}
