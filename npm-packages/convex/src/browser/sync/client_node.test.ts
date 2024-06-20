import child_process from "child_process";

import { test, expect } from "vitest";
import { Long } from "../long.js";

import { BaseConvexClient } from "./client.js";
import {
  ActionRequest,
  MutationRequest,
  parseServerMessage,
  RequestId,
  ServerMessage,
} from "./protocol.js";
import {
  encodeServerMessage,
  nodeWebSocket,
  withInMemoryWebSocket,
} from "./client_node_test_helpers.js";

test("BaseConvexClient protocol in node", async () => {
  await withInMemoryWebSocket(async ({ address, receive }) => {
    const client = new BaseConvexClient(
      address,
      () => {
        // ignore updates.
      },
      { webSocketConstructor: nodeWebSocket, unsavedChangesWarning: false },
    );

    expect((await receive()).type).toEqual("Connect");
    expect((await receive()).type).toEqual("ModifyQuerySet");

    await client.close();
  });
});

// Run the test above in its own Node.js subprocess to ensure that it exists
// cleanly. This is the only point of this test.
test("BaseConvexClient closes cleanly", () => {
  const p = child_process.spawnSync(
    "node_modules/.bin/vitest",
    [
      "-t",
      "BaseConvexClient protocol in node",
      "src/browser/sync/client_node.test.ts",
    ],
    {
      encoding: "utf-8",
      timeout: 35000,
      stdio: ["pipe", "pipe", "pipe"],
      env: {
        ...process.env,
        FORCE_COLOR: "false",
      },
    },
  );

  // If this is a timeout, the test didn't exit cleanly! Check for timers.
  expect(p.status).toBeFalsy();
});

test("Tests can encode longs in server messages", () => {
  const orig: ServerMessage = {
    type: "Transition",
    startVersion: { querySet: 0, identity: 0, ts: Long.fromNumber(0) },
    endVersion: { querySet: 1, identity: 0, ts: Long.fromNumber(1) },
    modifications: [
      {
        type: "QueryUpdated",
        queryId: 0,
        value: 0.0,
        logLines: ["[LOG] 'Got stuff'"],
      },
    ],
  };
  const encoded = encodeServerMessage(orig);
  const decoded = parseServerMessage(JSON.parse(encoded));
  expect(orig).toEqual(decoded);
});

// Detects an issue where actions sent before the WebSocket has connected
// are failed upon first connecting.
test("Actions can be called immediately", async () => {
  await withInMemoryWebSocket(async ({ address, receive, send }) => {
    const client = new BaseConvexClient(address, () => null, {
      webSocketConstructor: nodeWebSocket,
      unsavedChangesWarning: false,
    });
    const actionP = client.action("myAction", {});

    expect((await receive()).type).toEqual("Connect");
    expect((await receive()).type).toEqual("ModifyQuerySet");
    const actionRequest = await receive();
    expect(actionRequest.type).toEqual("Action");
    const requestId = (actionRequest as ActionRequest).requestId;

    send(actionSuccess(requestId));
    expect(await actionP).toBe(42);
    await client.close();
  });
});

function actionSuccess(requestId: RequestId): ServerMessage {
  return {
    type: "ActionResponse",
    requestId: requestId,
    success: true,
    result: 42,
    logLines: [],
  };
}

test("maxObservedTimestamp is updated on mutation and transition", async () => {
  await withInMemoryWebSocket(async ({ address, receive, send }) => {
    const client = new BaseConvexClient(address, () => null, {
      webSocketConstructor: nodeWebSocket,
      unsavedChangesWarning: false,
    });

    expect(client.getMaxObservedTimestamp()).toBeUndefined();

    const mutationP = client.mutation("myMutation", {});

    expect((await receive()).type).toEqual("Connect");
    expect((await receive()).type).toEqual("ModifyQuerySet");
    const mutationRequest = await receive();
    expect(mutationRequest.type).toEqual("Mutation");
    const requestId = (mutationRequest as MutationRequest).requestId;

    // Send a mutation, should update the max observed timestamp.
    send({
      type: "MutationResponse",
      requestId: requestId,
      success: true,
      result: 42,
      ts: Long.fromNumber(1000),
      logLines: [],
    });
    // Wait until getMaxObservedTimestamp() gets updated
    for (let i = 0; i < 10; i++) {
      if (client.getMaxObservedTimestamp()) {
        break;
      }
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    expect(client.getMaxObservedTimestamp()).toEqual(Long.fromNumber(1000));

    // Send a transition from before the mutation. Should not update the max
    // observed timestamp nor resolve the mutation.
    send({
      type: "Transition",
      startVersion: {
        querySet: 0,
        ts: Long.fromNumber(0),
        identity: 0,
      },
      endVersion: {
        querySet: 0,
        ts: Long.fromNumber(500),
        identity: 0,
      },
      modifications: [],
    });

    // Wait a bit and confirm the max timestamp has not been updated.
    await new Promise((resolve) => setTimeout(resolve, 200));
    expect(client.getMaxObservedTimestamp()).toEqual(Long.fromNumber(1000));

    // Send another transition with higher timestamp. This should resolve the
    // transition and advanced the max observable timestamp.
    send({
      type: "Transition",
      startVersion: {
        querySet: 0,
        ts: Long.fromNumber(500),
        identity: 0,
      },
      endVersion: {
        querySet: 0,
        ts: Long.fromNumber(2000),
        identity: 0,
      },
      modifications: [],
    });
    // Wait until the mutation is resolved.
    expect(await mutationP).toBe(42);
    expect(client.getMaxObservedTimestamp()).toEqual(Long.fromNumber(2000));

    await client.close();
  });
});
