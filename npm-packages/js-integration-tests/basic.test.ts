import { ConvexHttpClient } from "convex/browser";
import { ConvexReactClient } from "convex/react";
import { makeFunctionReference } from "convex/server";
import { api } from "./convex/_generated/api";
import { awaitQueryResult, opts } from "./test_helpers";
import { deploymentUrl } from "./common";

describe("HTTPClient", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  test("Client can be created", () => {
    // Just using the beforeEach hook.
  });

  test("Typos in UDF names should generate meaningful error messages", async () => {
    await expect(
      httpClient.query(makeFunctionReference<"query">("notARegisteredUdf"), {}),
    ).rejects.toThrow(
      "Could not find public function for 'notARegisteredUdf'. Did you forget to run `npx convex dev` or `npx convex deploy`?",
    );
  });

  test("Create single object via UDF", async () => {
    const empty_find = await httpClient.query(api.findObject.default);
    expect(empty_find).toBeNull();
    const binaryData = new ArrayBuffer(8);
    new DataView(binaryData).setBigUint64(0, 1017n, true);
    const stored = await httpClient.mutation(api.storeObject.default, {
      foo: "bar",
      baz: 42n,
      data: binaryData,
    });
    expect(stored).toHaveProperty("_id");
    expect(stored).toHaveProperty("baz", 42n);
    expect(stored).toHaveProperty("data", binaryData);
    const found = await httpClient.query(api.findObject.default);
    expect(found).toHaveProperty("data", binaryData);
  });

  test("Retrieve single object via UDF", async () => {
    const obj = await httpClient.mutation(api.storeObject.default, {
      foo: "baz",
    });
    const robj = await httpClient.query(api.getObject.default, { id: obj._id });
    expect(robj).toHaveProperty("foo", "baz");
  });

  test("Update single object via UDF", async () => {
    const obj = await httpClient.mutation(api.storeObject.default, { foo: 1 });
    let wobj = await httpClient.mutation(api.updateObject.default, {
      id: obj._id,
      field: "foo",
    });
    expect(wobj).toHaveProperty("foo", 2);
    wobj = await httpClient.mutation(api.updateObject.default, {
      id: obj._id,
      field: "foo",
    });
    expect(wobj).toHaveProperty("foo", 3);
    const robj = await httpClient.query(api.getObject.default, { id: obj._id });
    expect(robj).toHaveProperty("foo", 3);
  });

  test("Remove object via UDF", async () => {
    const obj = await httpClient.mutation(api.storeObject.default, { foo: 1 });
    await httpClient.mutation(api.updateObject.default, {
      id: obj._id,
      field: "foo",
    });
    const robj = await httpClient.query(api.getObject.default, { id: obj._id });
    expect(robj).toHaveProperty("foo", 2);
    await httpClient.mutation(api.removeObject.default, { id: obj._id });
    const noobj = await httpClient.query(api.getObject.default, {
      id: obj._id,
    });
    expect(noobj).toBeNull();
  });

  test("Reference data type", async () => {
    await httpClient.mutation(api.references.createGraph);
    const nodes = await httpClient.query(api.references.incomingEdges, {
      name: "a",
    });
    nodes.sort();
    expect(nodes).toEqual(["d", "e"]);
    await httpClient.mutation(api.references.deleteGraph);
  });

  test("Incrementally building up a graph", async () => {
    // Test the `Id`s going down to the client and back up to the server.
    const nodeA = await httpClient.mutation(api.references.addNode, {
      name: "a",
    });
    const nodeB = await httpClient.mutation(api.references.addNode, {
      name: "b",
    });
    const nodeC = await httpClient.mutation(api.references.addNode, {
      name: "c",
    });

    await httpClient.mutation(api.references.addEdge, {
      src: nodeA,
      dst: nodeC,
    });
    await httpClient.mutation(api.references.addEdge, {
      src: nodeB,
      dst: nodeC,
    });

    const nodes = await httpClient.query(api.references.incomingEdges, {
      name: "c",
    });
    expect(nodes).toEqual(["a", "b"]);
    await httpClient.mutation(api.references.deleteGraph);
  });

  test("Mutation failure", async () => {
    let err: null | Error = null;
    try {
      await httpClient.mutation(api.storeObject.throwError);
    } catch (e) {
      err = e as Error;
    }
    expect(err).not.toBeNull();
    expect(err!.toString()).toMatch(/Failure is temporary/);
  });
});

describe("ConvexReactClient", () => {
  let reactClient: ConvexReactClient;
  beforeEach(() => {
    reactClient = new ConvexReactClient(deploymentUrl, opts);
  });
  afterEach(async () => {
    await reactClient.mutation(api.cleanUp.default);
    await reactClient.close();
  });

  test("Subscribe to object", async () => {
    const obj = await reactClient.mutation(api.storeObject.default, {
      money: 11,
    });
    const watch = reactClient.watchQuery(api.getObject.default, {
      id: obj._id,
    });

    const docWith14Money = awaitQueryResult(watch, (doc) => doc.money === 14);

    await reactClient.mutation(api.updateObject.default, {
      id: obj._id,
      field: "money",
    });
    await reactClient.mutation(api.updateObject.default, {
      id: obj._id,
      field: "foo",
    });
    await reactClient.mutation(api.updateObject.default, {
      id: obj._id,
      field: "money",
    });
    await reactClient.mutation(api.updateObject.default, {
      id: obj._id,
      field: "bar",
    });
    await reactClient.mutation(api.updateObject.default, {
      id: obj._id,
      field: "money",
    });
    const finalDoc = await docWith14Money;
    expect(finalDoc.money).toEqual(14);
  });
});
