import {
  ConvexClient,
  setDefaultWebSocketConstructor,
} from "./simple_client.js";

// Prefer the native global WebSocket when available (Bun, Deno, Node >= 21)
// and fall back to the `ws` npm package for older Node.js versions.
// This file is compiled with `bundle: true` with an exception for
// `./simple_client.js` so a dynamic import approach won't work here.
import ws from "ws";
const nodeWebSocket: typeof WebSocket =
  typeof WebSocket !== "undefined"
    ? WebSocket
    : (ws as unknown as typeof WebSocket);

setDefaultWebSocketConstructor(nodeWebSocket);

export { ConvexClient };
