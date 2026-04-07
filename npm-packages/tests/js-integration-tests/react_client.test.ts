import { ConvexReactClient } from "convex/react";
import { api } from "./convex/_generated/api";
import { opts } from "./test_helpers";
import { deploymentUrl } from "./common";

let client: ConvexReactClient;

beforeEach(() => {
  client = new ConvexReactClient(deploymentUrl, opts);
});
afterEach(async () => {
  await client.mutation(api.cleanUp.default);
  await client.close();
});

describe("Single-shot queries", () => {
  test("don't update", async () => {
    const queryPromise = client.query(api.getUsers.default);
    expect(await queryPromise).toEqual([]);

    await client.mutation(api.addUser.default, { name: "john" });

    expect(await queryPromise).toEqual([]);
    expect(await client.query(api.getUsers.default)).toHaveLength(1);
  });

  test("throw errors", async () => {
    await expect(() => client.query(api.error.default)).rejects.toThrow();
  });
});
