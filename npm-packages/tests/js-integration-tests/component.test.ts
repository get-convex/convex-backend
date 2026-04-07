import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api";
import { deploymentUrl } from "./common";

describe("Basic component tests", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  test("Failed child mutation rolls back writes", async () => {
    // Internally, partialRollback asserts that partial
    // commits are visible to the parent transaction, while rollbacks aren't.
    await httpClient.mutation(api.messages.partialRollback);
    // Now we check what actually got committed.
    const committedMessages = await httpClient.query(
      api.messages.messagesInComponent,
    );
    expect(committedMessages).toEqual(["hello buddy"]);
  });

  test("Function handles are accessible from queries and actions", async () => {
    const queryHandles = await httpClient.query(
      api.component.functionHandleQuery,
      {},
    );
    expect(typeof queryHandles.appHandle).toEqual("string");
    expect(typeof queryHandles.componentHandle).toEqual("string");

    const actionHandles = await httpClient.action(
      api.component.functionHandleAction,
      {},
    );
    expect(typeof actionHandles.appHandle).toEqual("string");
    expect(typeof actionHandles.componentHandle).toEqual("string");

    expect(queryHandles.appHandle).toEqual(actionHandles.appHandle);
    expect(queryHandles.componentHandle).toEqual(actionHandles.componentHandle);
  });

  test("Function handles can be used in calling APIs", async () => {
    await httpClient.query(api.component.queryCallsHandles, {});
    await httpClient.mutation(api.component.mutationCallsHandles, {});
    await httpClient.action(api.component.actionCallsHandles, {});
  });

  test("Function handles can be used in scheduling APIs", async () => {
    await httpClient.mutation(api.component.passHandleToScheduler, {});
  });
});
