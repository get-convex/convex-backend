import { BaseConvexClient } from "convex/browser";

type OutgoingMessage =
  | { type: "connect"; webSocketId: number }
  | {
      type: "send";
      webSocketId: number;
      data: string;
    }
  | { type: "close"; webSocketId: number };
const outgoingMessages: OutgoingMessage[] = [];

type IncomingMessage =
  | { type: "connected"; webSocketId: number }
  | { type: "message"; webSocketId: number; data: string }
  | { type: "closed"; webSocketId: number };

let nextWebSocketId = 0;
const webSockets = new Map<number, TestingWebSocket>();

class TestingWebSocket {
  websocketId: number;

  onopen?: (this: TestingWebSocket, ev: Event) => any;
  onerror?: (this: TestingWebSocket, ev: Event) => any;
  onmessage?: (this: TestingWebSocket, ev: MessageEvent) => any;
  onclose?: (this: TestingWebSocket, ev: CloseEvent) => any;

  constructor(_url: string | URL, _protocols?: string | string[]) {
    this.websocketId = nextWebSocketId++;
    webSockets.set(this.websocketId, this);
    outgoingMessages.push({ type: "connect", webSocketId: this.websocketId });
  }

  send(data: string | ArrayBuffer | Blob | ArrayBufferView) {
    if (typeof data !== "string") {
      throw new Error("Only strings are supported");
    }
    outgoingMessages.push({
      type: "send",
      webSocketId: this.websocketId,
      data,
    });
  }

  close() {
    outgoingMessages.push({ type: "close", webSocketId: this.websocketId });
  }
}

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
      default: {
        const _: never = message;
      }
    }
  }
}

export function getMaxObservedTimestamp() {
  return client.getMaxObservedTimestamp()?.toString();
}

const address = "https://suadero.example.com";
export const client = new BaseConvexClient(address, () => {}, {
  unsavedChangesWarning: false,
  skipConvexDeploymentUrlCheck: true,
  webSocketConstructor: TestingWebSocket as any,
});
