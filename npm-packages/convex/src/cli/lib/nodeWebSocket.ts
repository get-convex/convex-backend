import ws from "ws";

// Prefer the native global WebSocket when available (Bun, Deno, Node >= 21)
// and fall back to the `ws` npm package for older Node.js versions.
// This fixes compatibility issues where Bun's `ws` shim mishandles the HTTP
// 101 upgrade handshake, especially behind reverse proxies.
// See: https://github.com/get-convex/convex-backend/issues/390
export const nodeWebSocket: typeof WebSocket =
  typeof WebSocket !== "undefined"
    ? WebSocket
    : (ws as unknown as typeof WebSocket);
