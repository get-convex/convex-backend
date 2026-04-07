import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api";
import { EXAMPLE_DATA } from "./foodData";
import { deploymentUrl } from "./common";

describe("HTTPClient", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(async () => {
    httpClient = new ConvexHttpClient(deploymentUrl);
    await httpClient.action(api.mountedSearch.populateFoods);
  });

  afterEach(async () => {
    await httpClient.mutation(api.mountedSearch.cleanUp);
  });

  test("Run a text search in a query", async () => {
    const result = await httpClient.query(
      api.mountedSearch.fullTextSearchQuery,
      {
        query: "al pastor",
      },
    );
    expect(result[0].description).toStrictEqual(EXAMPLE_DATA[0].description);
  });

  test("Run a text search with a filter in a query", async () => {
    const result = await httpClient.query(
      api.mountedSearch.fullTextSearchQuery,
      {
        query: "al pastor",
        cuisine: "mexican",
      },
    );
    expect(result[0].description).toStrictEqual(EXAMPLE_DATA[0].description);
  });

  test("Run a text search with a non-matching filter in a query", async () => {
    const result = await httpClient.query(
      api.mountedSearch.fullTextSearchQuery,
      {
        query: "al pastor",
        cuisine: EXAMPLE_DATA[0].cuisine + "no",
      },
    );
    expect(result).toStrictEqual([]);
  });

  test("Run a text search in a mutation", async () => {
    const result = await httpClient.mutation(
      api.mountedSearch.fullTextSearchMutation,
      {
        query: "al pastor",
      },
    );
    expect(result[0].description).toStrictEqual(EXAMPLE_DATA[0].description);
  });

  test("Run a text search with a write in a mutation", async () => {
    const result = await httpClient.mutation(
      api.mountedSearch.fullTextSearchMutationWithWrite,
      {
        query: "al pastor",
      },
    );
    expect(result.map((value: any) => value.description)).toStrictEqual([
      EXAMPLE_DATA[0].description,
      EXAMPLE_DATA[0].description,
    ]);
  });

  test("Run a text search with a filter in a mutation", async () => {
    const result = await httpClient.mutation(
      api.mountedSearch.fullTextSearchMutation,
      {
        query: "al pastor",
        cuisine: EXAMPLE_DATA[0].cuisine,
      },
    );
    expect(result[0].description).toStrictEqual(EXAMPLE_DATA[0].description);
  });

  test("Run a text search with a non-matching filter in a mutation", async () => {
    const result = await httpClient.mutation(
      api.mountedSearch.fullTextSearchMutation,
      {
        query: "al pastor",
        cuisine: EXAMPLE_DATA[0].cuisine + "no",
      },
    );
    expect(result).toStrictEqual([]);
  });
});
