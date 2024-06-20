import { test, expect } from "vitest";
import { withInMemoryWebSocket } from "./sync/client_node_test_helpers.js";
import { DefaultFunctionArgs, makeFunctionReference } from "../server/index.js";
// This Node.js build sets up the WebSocket dependency automatically.
import { ConvexClient } from "./simple_client-node.js";

test("Subscriptions are deduplicated", async () => {
  // The actual implementation of this dedupliation logic is in BaseConvexClient.
  await withInMemoryWebSocket(async ({ address, receive }) => {
    const client = new ConvexClient(address, {
      unsavedChangesWarning: false,
    });
    expect((await receive()).type).toEqual("Connect");
    expect((await receive()).type).toEqual("ModifyQuerySet");

    const apiQueryFunc = makeFunctionReference<
      "query",
      DefaultFunctionArgs,
      string
    >("jeans style");
    const apiMutationFunc = makeFunctionReference<
      "mutation",
      DefaultFunctionArgs,
      string
    >("jeans style");
    const { unsubscribe: unsub1 } = client.onUpdate(
      apiQueryFunc,
      {},
      () => null,
    );
    expect((await receive()).type).toEqual("ModifyQuerySet");

    // The second subscribe to the same query should not send another query request.
    const { unsubscribe: unsub2 } = client.onUpdate(
      apiQueryFunc,
      {},
      () => null,
    );
    unsub1();
    void client.mutation(apiMutationFunc, {});
    // Receiving a mutation next means no third or fourth ModifyQuerySet.
    expect((await receive()).type).toEqual("Mutation");

    // Unsubscribing the second time should trigger the unsubscribe.
    unsub2();
    expect((await receive()).type).toEqual("ModifyQuerySet");

    await client.close();
  });
});
