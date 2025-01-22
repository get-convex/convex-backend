// This file can be combined with ./basic.test.ts once these APIs are public.

import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api";
import { EXAMPLE_DATA } from "./foodData";
import { deploymentUrl } from "./common";

describe("HTTPClient", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(async () => {
    httpClient = new ConvexHttpClient(deploymentUrl);
    await httpClient.action(api.foods.populate);
  });

  test("Run a node based vector search", async () => {
    const result = await httpClient.action(api.vectorActionNode.vectorSearch, {
      embedding: EXAMPLE_DATA[0].embedding,
      cuisine: EXAMPLE_DATA[0].cuisine,
    });
    expect(result[0].description).toStrictEqual(EXAMPLE_DATA[0].description);
  });

  test("Run a v8 based vector search", async () => {
    const result = await httpClient.action(api.vectorActionV8.vectorSearch, {
      embedding: EXAMPLE_DATA[0].embedding,
      cuisine: EXAMPLE_DATA[0].cuisine,
    });
    expect(result[0].description).toStrictEqual(EXAMPLE_DATA[0].description);
  });
});
