import { Base64 } from "../../values/index.js";
import { Long } from "../long.js";

// --experimental-vm-modules which we use for jest doesn't support named exports
import ws from "ws";

// Let's pretend this ws WebSocket is a browser WebSocket (it's very close)
export const nodeWebSocket = ws as unknown as typeof WebSocket;

import { ClientMessage, ServerMessage } from "./protocol.js";

export type InMemoryWebSocketTest = (args: {
  address: string;
  socket: () => WebSocket;
  receive: () => Promise<ClientMessage>;
  send: (message: ServerMessage) => void;
  close: () => void;
}) => Promise<void>;

// Run a test with a real node WebSocket instance connected
export async function withInMemoryWebSocket(
  cb: InMemoryWebSocketTest,
  debug = false,
) {
  let wss = new ws.WebSocketServer({ port: 0 });

  let received: (msg: string) => void;
  const messages: Promise<string>[] = [
    new Promise((r) => {
      received = r;
    }),
  ];
  let socket: ws.WebSocket | null = null;
  const setupSocket = () => {
    wss.on("connection", function connection(ws: ws.WebSocket) {
      socket = ws;
      ws.on("message", function message(data: string) {
        received(data);
        if (debug) console.debug(`client --${JSON.parse(data).type}--> `);
        messages.push(
          new Promise((r) => {
            received = r;
          }),
        );
      });
    });
  };
  setupSocket();
  async function receive(): Promise<ClientMessage> {
    const msgP = messages.shift();
    if (!msgP) {
      throw new Error("Receive() called twice? No message promise found.");
    }
    return JSON.parse(await msgP);
  }
  function send(message: ServerMessage) {
    if (debug) console.debug(`      <--${message.type}-- server`);
    socket!.send(encodeServerMessage(message));
  }

  const s = wss.address();
  const address = typeof s === "string" ? s : `http://127.0.0.1:${s.port}`;

  try {
    await cb({
      address,
      socket: () => socket as unknown as WebSocket,
      receive,
      send,
      close: () => {
        socket!.close();
        wss.close();
        wss = new ws.WebSocketServer({ port: (s as ws.AddressInfo).port });
        setupSocket();
      },
    });
  } finally {
    socket!.close();
    wss.close();
  }
}

export function encodeServerMessage(message: ServerMessage): string {
  function replacer(_key: string, value: any) {
    if (Long.isLong(value)) {
      return encodeLong(value);
    }
    return value;
  }
  return JSON.stringify(message, replacer);
}

function encodeLong(n: Long) {
  const integerBytes = Uint8Array.from(n.toBytesLE());
  return Base64.fromByteArray(integerBytes);
}
