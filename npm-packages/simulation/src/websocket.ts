import { BaseConvexClient } from "convex/browser";
import { outgoingMessages } from "./protocol";

let nextWebSocketId = 0;
export const webSockets = new Map<number, TestingWebSocket>();

export class TestingWebSocket {
  websocketId: number;

  onopen?: (this: TestingWebSocket, ev: Event) => any;
  onerror?: (this: TestingWebSocket, ev: Event) => any;
  onmessage?: (this: TestingWebSocket, ev: MessageEvent) => any;
  onclose?: (this: TestingWebSocket, ev: CloseEvent) => any;

  constructor(_url: string | URL, _protocols?: string | string[]) {
    this.websocketId = nextWebSocketId++;
    webSockets.set(this.websocketId, this);
    console.log("WebSocket connected");
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

export function getMaxObservedTimestamp() {
  return client.getMaxObservedTimestamp()?.toString();
}

const address = "https://suadero.example.com";
export const client = new BaseConvexClient(address, () => {}, {
  unsavedChangesWarning: false,
  skipConvexDeploymentUrlCheck: true,
  webSocketConstructor: TestingWebSocket as any,
});
