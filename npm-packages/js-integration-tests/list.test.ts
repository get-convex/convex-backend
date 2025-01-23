import { ConvexReactClient } from "convex/react";
import { api } from "./convex/_generated/api";
import { Doc } from "./convex/_generated/dataModel";
import { awaitQueryResult, opts } from "./test_helpers";
import { deploymentUrl } from "./common";

let client: ConvexReactClient;

beforeEach(() => {
  client = new ConvexReactClient(deploymentUrl, opts);
});
afterEach(async () => {
  await client.mutation(api.cleanUp.default);
  await client.close();
});

test("Subscribe to a table", async () => {
  const watch = client.watchQuery(api.getUsers.default, {});

  const resultWith4Users = awaitQueryResult(
    watch,
    (result) => result.length === 4,
  );
  await client.mutation(api.addUser.default, { name: "john" });
  await client.mutation(api.addUser.default, { name: "paul" });
  await client.mutation(api.addUser.default, { name: "george" });
  await client.mutation(api.addUser.default, { name: "ringo" });
  const result: Doc<"users">[] = await resultWith4Users;
  expect(result.map((doc) => doc.name)).toEqual([
    "john",
    "paul",
    "george",
    "ringo",
  ]);
});
