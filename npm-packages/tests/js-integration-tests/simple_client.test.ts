import { ConvexClient } from "convex/browser";
import { api } from "./convex/_generated/api";
import { Doc } from "./convex/_generated/dataModel";
import { AsyncQueue, defer } from "./test_helpers";
import { deploymentUrl } from "./common";

describe("Callback-based JavaScript client", () => {
  let client: ConvexClient;

  beforeEach(() => {
    client = new ConvexClient(deploymentUrl);
  });
  afterEach(async () => {
    await client.mutation(api.cleanUp.default, {});
    await client.close();
  });

  test("Subscribing to a query", async () => {
    const updates = new AsyncQueue<
      (typeof api.getUsers.default)["_returnType"]
    >();

    const { getCurrentValue } = client.onUpdate(api.getUsers.default, {}, (v) =>
      updates.push(v),
    );
    expect(getCurrentValue()).toEqual(undefined);
    expect(await updates.shift()).toEqual([]);
    expect(updates.queue.length).toEqual(0);

    await client.mutation(api.addUser.default, { name: "john" });
    // This mutation should not resolve until all query callbacks have fired
    // so the response should be available immediately.
    expect(updates.queue.length).toEqual(1);

    await client.mutation(api.addUser.default, { name: "paul" });
    await client.mutation(api.addUser.default, { name: "george" });
    await client.mutation(api.addUser.default, { name: "ringo" });

    await updates.shift();
    await updates.shift();
    await updates.shift();

    const result: Doc<"users">[] = await updates.shift();
    expect(result.map((doc) => doc.name)).toEqual([
      "john",
      "paul",
      "george",
      "ringo",
    ]);
    expect(getCurrentValue()).toHaveLength(4);
  });

  test("Single-shot query rejects", async () => {
    await expect(client.query(api.getUsers.default, {})).resolves.toEqual([]);
    await expect(client.query(api.error.default, {})).rejects.toThrow();
  });

  test("onSubscribe calls the provided onError callback when the query errors", async () => {
    const { promise, resolve, reject } = defer();
    const unsubscribe = client.onUpdate(api.error.default, {}, resolve, reject);
    try {
      await promise;
      throw new Error("Failed query did not throw.");
    } catch (e: any) {
      expect(e.message).toContain("oopsie");
    }
    unsubscribe();
  });

  // Batching here means that callbacks scheduled in microtasks that run during
  // the same tick will run synchronously one after another.
  test("multiple callbacks to already-subscribed queries are batched", async () => {
    const a = defer<any>("a");
    const b = defer<any>("b");
    const c = defer<any>("c");
    const d = defer<any>("d");

    client.onUpdate(api.cachebust.default, { cacheBust: 1 }, a.resolve);
    client.onUpdate(api.cachebust.default, { cacheBust: 2 }, b.resolve);
    // wait for them both to have results locally
    await Promise.all([a.promise, b.promise]);

    // Now that both queries has results locally, new callbacks registered should run very soon
    // after they are scheduled.
    // They should still be batched though.
    // And this works even if the callbacks are scheduled during different microtasks!
    client.onUpdate(api.cachebust.default, { cacheBust: 1 }, c.resolve);
    await Promise.resolve().then(() => {
      client.onUpdate(api.cachebust.default, { cacheBust: 2 }, d.resolve);
    });

    expect(c.resolved).toBe(false);
    expect(d.resolved).toBe(false);

    // Once has run, the other has too.
    await c.promise;
    expect(d.resolved).toBe(true);
  });
});
