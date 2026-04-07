import { ConvexReactClient } from "convex/react";
import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api";
import { opts } from "./test_helpers";
import { deploymentUrl } from "./common";

describe("Sets handling", () => {
  const httpClient = new ConvexHttpClient(deploymentUrl);
  const reactClient = new ConvexReactClient(deploymentUrl, opts);

  afterAll(async () => {
    await reactClient.close();
  });

  test("Cannot return a Set from a query", async () => {
    await expectErrorContains(
      httpClient.query(api.sets.listValues),
      "Set[] is not a supported Convex type",
    );
  });

  test("Cannot return a Set from a mutation", async () => {
    await expectErrorContains(
      httpClient.mutation(api.sets.mutationReturningSet),
      'Set["hello","world"] is not a supported Convex type',
    );
  });

  test("Cannot return a Set from an action", async () => {
    await expectErrorContains(
      httpClient.action(api.sets.actionReturningSet),
      'Set["hello","world"] is not a supported Convex type',
    );
  });

  test("Cannot return a Set from a Node.js action", async () => {
    await expectErrorContains(
      httpClient.action(api.actions.simple.returnSet),
      'Set["hello","world"] is not a supported Convex type',
    );
  });

  test("Cannot write a Set in a mutation", async () => {
    await expectErrorContains(
      httpClient.mutation(api.sets.insertValue),
      "Set[1,2] is not a supported Convex type",
    );
  });

  test("Cannot pass Set as an argument to a query", async () => {
    await expectErrorContains(
      httpClient.query(api.sets.queryWithAnyArg, { x: new Set(["bla"]) }),
      'Set["bla"] is not a supported Convex type',
    );
  });

  test("Cannot pass Set as an argument to a mutation", async () => {
    await expectErrorContains(
      httpClient.mutation(api.sets.mutationWithAnyArg, { x: new Set(["bla"]) }),
      'Set["bla"] is not a supported Convex type',
    );
  });

  test("Cannot pass Set as an argument to an action", async () => {
    await expectErrorContains(
      httpClient.action(api.sets.actionWithAnyArg, { x: new Set(["bla"]) }),
      'Set["bla"] is not a supported Convex type',
    );
  });

  test("Cannot pass Set as an argument to a query in React", async () => {
    await expectErrorContains(
      reactClient.query(api.sets.queryWithAnyArg, { x: new Set(["bla"]) }),
      'Set["bla"] is not a supported Convex type',
    );
  });

  test("Cannot pass Set as an argument to a mutation in React", async () => {
    await expectErrorContains(
      reactClient.mutation(api.sets.mutationWithAnyArg, {
        x: new Set(["bla"]),
      }),
      'Set["bla"] is not a supported Convex type',
    );
  });

  test("Cannot pass Set as an argument to an action in React", async () => {
    await expectErrorContains(
      reactClient.action(api.sets.actionWithAnyArg, { x: new Set(["bla"]) }),
      'Set["bla"] is not a supported Convex type',
    );
  });
});

describe("Maps handling", () => {
  const httpClient = new ConvexHttpClient(deploymentUrl);
  const reactClient = new ConvexReactClient(deploymentUrl, opts);

  afterAll(async () => {
    await reactClient.close();
  });

  test("Cannot return a Map from a query", async () => {
    await expectErrorContains(
      httpClient.query(api.maps.listValues),
      "Map[] is not a supported Convex type",
    );
  });

  test("Cannot return a Map from a mutation", async () => {
    await expectErrorContains(
      httpClient.mutation(api.maps.mutationReturningMap),
      'Map[["key","value"]] is not a supported Convex type',
    );
  });

  test("Cannot return a Map from an action", async () => {
    await expectErrorContains(
      httpClient.action(api.maps.actionReturningMap),
      'Map[["key","value"]] is not a supported Convex type',
    );
  });

  test("Cannot return a Map from a Node.js action", async () => {
    await expectErrorContains(
      httpClient.action(api.actions.simple.returnMap),
      'Map[["key","value"]] is not a supported Convex type',
    );
  });

  test("Cannot write a Map in a mutation", async () => {
    await expectErrorContains(
      httpClient.mutation(api.maps.createMap),
      'Map[["n","m"]] is not a supported Convex type',
    );
  });

  test("Cannot pass Map as an argument to a query", async () => {
    await expectErrorContains(
      httpClient.query(api.sets.queryWithAnyArg, { x: new Map([["k", "v"]]) }),
      'Map[["k","v"]] is not a supported Convex type',
    );
  });

  test("Cannot pass Map as an argument to a mutation", async () => {
    await expectErrorContains(
      httpClient.mutation(api.sets.mutationWithAnyArg, {
        x: new Map([["k", "v"]]),
      }),
      'Map[["k","v"]] is not a supported Convex type',
    );
  });

  test("Cannot pass Map as an argument to an action", async () => {
    await expectErrorContains(
      httpClient.action(api.sets.actionWithAnyArg, {
        x: new Map([["k", "v"]]),
      }),
      'Map[["k","v"]] is not a supported Convex type',
    );
  });

  test("Cannot pass Map as an argument to a query in React", async () => {
    await expectErrorContains(
      reactClient.query(api.sets.queryWithAnyArg, { x: new Map([["k", "v"]]) }),
      'Map[["k","v"]] is not a supported Convex type',
    );
  });

  test("Cannot pass Map as an argument to a mutation in React", async () => {
    await expectErrorContains(
      reactClient.mutation(api.sets.mutationWithAnyArg, {
        x: new Map([["k", "v"]]),
      }),
      'Map[["k","v"]] is not a supported Convex type',
    );
  });

  test("Cannot pass Map as an argument to an action in React", async () => {
    await expectErrorContains(
      reactClient.action(api.sets.actionWithAnyArg, {
        x: new Map([["k", "v"]]),
      }),
      'Map[["k","v"]] is not a supported Convex type',
    );
  });
});

async function expectErrorContains(source: Promise<any>, message: string) {
  await expect(source).rejects.toHaveProperty(
    "message",
    expect.stringContaining(message),
  );
}
