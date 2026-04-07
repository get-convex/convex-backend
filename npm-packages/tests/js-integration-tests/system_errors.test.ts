import { ConvexHttpClient } from "convex/browser";
import { ConvexReactClient } from "convex/react";
import { api } from "./convex/_generated/api";
import { opts } from "./test_helpers";
import { deploymentUrl } from "./common";

describe("HTTPClient", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  test("call occ error", async () => {
    await expect(httpClient.mutation(api.error.occ)).rejects.toThrow(
      "OptimisticConcurrencyControlFailure",
    );
  });

  test("call overloaded error", async () => {
    await expect(httpClient.mutation(api.error.overloaded)).rejects.toThrow(
      "Busy",
    );
  });
});

describe("ConvexReactClient", () => {
  let reactClient: ConvexReactClient;
  let httpClient: ConvexHttpClient;
  beforeEach(() => {
    reactClient = new ConvexReactClient(deploymentUrl, opts);
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await reactClient.close();
    await httpClient.mutation(api.cleanUp.default);
  });

  test("call occ error", async () => {
    // expect this to retry until there's a message.
    const promise = reactClient.mutation(api.error.occ);

    // Send a mutation out of band via http client to create a message
    // react client is busy crash looping its websocket
    await httpClient.mutation(api.messages.sendMessage, {
      channel: "hi",
      text: "there",
    });
    await promise;
  });

  test("call overloaded error", async () => {
    // expect this to retry until there's a message.
    const promise = reactClient.mutation(api.error.overloaded);

    // Send a mutation out of band via http client to create a message
    // react client is busy crash looping its websocket
    await httpClient.mutation(api.messages.sendMessage, {
      channel: "hi",
      text: "there",
    });
    await promise;
  });
});
