import { test, expect } from "vitest";
import { Long } from "../browser/long.js";

import ReactDOM from "react-dom";

import { ConvexReactClient } from "./client.js";
import {
  ClientMessage,
  QuerySetModification,
  ServerMessage,
} from "../browser/sync/protocol.js";
import {
  nodeWebSocket,
  withInMemoryWebSocket,
} from "../browser/sync/client_node_test_helpers.js";
import { anyApi } from "../server/api.js";

const testReactClient = (address: string) =>
  new ConvexReactClient(address, {
    webSocketConstructor: nodeWebSocket,
    unsavedChangesWarning: false,
  });

test("ConvexReactClient ends subscriptions on close", async () => {
  await withInMemoryWebSocket(async ({ address, receive, send }) => {
    const client = testReactClient(address);
    const watch = client.watchQuery(anyApi.myQuery.default, {});
    let timesCallbackRan = 0;
    let timesReactScheduled = 0;
    watch.onUpdate(() => timesCallbackRan++);

    expect((await receive()).type).toEqual("Connect");
    const modify = expectQuerySetModification(await receive());
    expect(modify.modifications).toEqual([
      {
        args: [{}],
        queryId: 0,
        type: "Add",
        udfPath: "myQuery:default",
      },
    ]);
    expect(timesCallbackRan).toEqual(0);

    send(transition());

    // Monkey-patch to mock out this react-dom function.
    // Mocking in Jest like `jest.mock('react-dom')` doesn't work with ESM yet.
    const orig = ReactDOM.unstable_batchedUpdates;
    try {
      const scheduledCallback = await new Promise<() => void>((resolve) => {
        ReactDOM.unstable_batchedUpdates = function mock(cb: any) {
          timesReactScheduled++;
          resolve(cb);
        };
      });
      expect(timesReactScheduled).toEqual(1);

      // After the callback has been registered with unstable_batchedUpdates but
      // before the callback has been run, close the client.
      const closePromise = client.close();

      // Later, React calls the callback. This should do nothing.
      scheduledCallback();
      expect(timesCallbackRan).toEqual(0);

      // After the internal client has closed, same nothing.
      await closePromise;
      scheduledCallback();
      expect(timesCallbackRan).toEqual(0);
    } finally {
      ReactDOM.unstable_batchedUpdates = orig;
    }
  });
});

const expectQuerySetModification = (
  message: ClientMessage,
): QuerySetModification => {
  expect(message.type).toEqual("ModifyQuerySet");
  if (message.type !== "ModifyQuerySet") throw new Error("Wrong message!");
  return message;
};

function transition(): ServerMessage {
  return {
    type: "Transition",
    startVersion: { querySet: 0, identity: 0, ts: Long.fromNumber(0) },
    endVersion: { querySet: 1, identity: 0, ts: Long.fromNumber(1) },
    modifications: [
      {
        type: "QueryUpdated",
        queryId: 0,
        value: 0.0,
        logLines: [],
        journal: null,
      },
    ],
  };
}
