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

  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  test("Run a text search in a query", async () => {
    const result = await httpClient.query(api.textSearch.fullTextSearchQuery, {
      query: "al pastor",
    });
    expect(result[0].description).toStrictEqual(EXAMPLE_DATA[0].description);
  });

  test("Run a text search without fuzzy match in a query", async () => {
    const result = await httpClient.query(api.textSearch.fullTextSearchQuery, {
      query: "past1r",
    });
    expect(result.length).toStrictEqual(0);
  });

  test("Run a text search with prefix match in a query", async () => {
    const result = await httpClient.query(api.textSearch.fullTextSearchQuery, {
      query: "pasto",
    });
    expect(result[0].description).toStrictEqual(EXAMPLE_DATA[0].description);
  });

  test("Run a paginated text search in a query", async () => {
    const firstPage = await httpClient.query(
      api.textSearch.paginatedFullTextSearchQuery,
      {
        query: "past",
        paginationOptions: {
          numItems: 1,
          cursor: null,
        },
      },
    );
    expect(firstPage.page[0].description).toStrictEqual(
      EXAMPLE_DATA[0].description,
    );
    const secondPage = await httpClient.query(
      api.textSearch.paginatedFullTextSearchQuery,
      {
        query: "past",
        paginationOptions: {
          numItems: 1,
          cursor: firstPage.continueCursor,
        },
      },
    );
    expect(secondPage.page[0].description).toStrictEqual(
      EXAMPLE_DATA[1].description,
    );
  });

  test("Run a text search with a filter in a query", async () => {
    const result = await httpClient.query(api.textSearch.fullTextSearchQuery, {
      query: "al pastor",
      cuisine: "mexican",
    });
    expect(result[0].description).toStrictEqual(EXAMPLE_DATA[0].description);
  });

  test("Run a text search with several filters in a query", async () => {
    const result = await httpClient.query(
      api.textSearch.fullTextSearchQuerySeveralFilters,
      {
        query: "al pastor",
        theLetterA: "a",
        cuisine: "mexican",
        bOrC: "b",
      },
    );
    expect(result[0].description).toStrictEqual(EXAMPLE_DATA[0].description);
  });

  test("Run a text search with several filters in a query that returns nothing", async () => {
    const result = await httpClient.query(
      api.textSearch.fullTextSearchQuerySeveralFilters,
      {
        query: "al pastor",
        theLetterA: "MATCHES NOTHING",
        cuisine: "mexican",
        bOrC: "b",
      },
    );
    expect(result).toHaveLength(0);
  });

  test("Run a paginated text search with a filter in a query", async () => {
    const firstPage = await httpClient.query(
      api.textSearch.paginatedFullTextSearchQuery,
      {
        query: "past",
        cuisine: "mexican",
        paginationOptions: {
          numItems: 1,
          cursor: null,
        },
      },
    );
    // Ideally this would be false to save one more round trip, but as long as
    // the next query is empty and done, it's probably ok.
    expect(firstPage.isDone).toStrictEqual(false);
    expect(firstPage.page.length).toStrictEqual(1);
    expect(firstPage.page[0].description).toStrictEqual(
      EXAMPLE_DATA[0].description,
    );
    const secondPage = await httpClient.query(
      api.textSearch.paginatedFullTextSearchQuery,
      {
        query: "past",
        cuisine: "mexican",
        paginationOptions: {
          numItems: 1,
          cursor: firstPage.continueCursor,
        },
      },
    );
    expect(secondPage.isDone).toStrictEqual(true);
    expect(secondPage.page).toStrictEqual([]);
  });

  test("Run a text search with a non-matching filter in a query", async () => {
    const result = await httpClient.query(api.textSearch.fullTextSearchQuery, {
      query: "al pastor",
      cuisine: EXAMPLE_DATA[0].cuisine + "no",
    });
    expect(result).toStrictEqual([]);
  });

  test("Run a text search in a mutation", async () => {
    const result = await httpClient.mutation(
      api.textSearch.fullTextSearchMutation,
      {
        query: "al pastor",
      },
    );
    expect(result[0].description).toStrictEqual(EXAMPLE_DATA[0].description);
  });

  test("Run a text search with a write in a mutation", async () => {
    const result = await httpClient.mutation(
      api.textSearch.fullTextSearchMutationWithWrite,
      {
        query: "al pastor",
      },
    );
    expect(result.map((value) => value.description)).toStrictEqual([
      EXAMPLE_DATA[0].description,
      EXAMPLE_DATA[0].description,
    ]);
  });

  test("Run a paginated text search in a mutation", async () => {
    const firstPage = await httpClient.mutation(
      api.textSearch.paginatedFullTextSearchMutation,
      {
        query: "past",
        paginationOptions: {
          numItems: 1,
          cursor: null,
        },
      },
    );
    expect(firstPage.page[0].description).toStrictEqual(
      EXAMPLE_DATA[0].description,
    );
    const secondPage = await httpClient.mutation(
      api.textSearch.paginatedFullTextSearchMutation,
      {
        query: "past",
        paginationOptions: {
          numItems: 1,
          cursor: firstPage.continueCursor,
        },
      },
    );
    expect(secondPage.page[0].description).toStrictEqual(
      EXAMPLE_DATA[1].description,
    );
  });

  test("Run a text search with a filter in a mutation", async () => {
    const result = await httpClient.mutation(
      api.textSearch.fullTextSearchMutation,
      {
        query: "al pastor",
        cuisine: EXAMPLE_DATA[0].cuisine,
      },
    );
    expect(result[0].description).toStrictEqual(EXAMPLE_DATA[0].description);
  });

  test("Run a paginated text search with a filter in a mutation", async () => {
    const firstPage = await httpClient.mutation(
      api.textSearch.paginatedFullTextSearchMutation,
      {
        query: "past",
        cuisine: "mexican",
        paginationOptions: {
          numItems: 1,
          cursor: null,
        },
      },
    );
    // Ideally this would be false to save one more round trip, but as long as
    // the next query is empty and done, it's probably ok.
    expect(firstPage.isDone).toStrictEqual(false);
    expect(firstPage.page.length).toStrictEqual(1);
    expect(firstPage.page[0].description).toStrictEqual(
      EXAMPLE_DATA[0].description,
    );
    const secondPage = await httpClient.mutation(
      api.textSearch.paginatedFullTextSearchMutation,
      {
        query: "past",
        cuisine: "mexican",
        paginationOptions: {
          numItems: 1,
          cursor: firstPage.continueCursor,
        },
      },
    );
    expect(secondPage.isDone).toStrictEqual(true);
    expect(secondPage.page).toStrictEqual([]);
  });

  test("Run a text search with a non-matching filter in a mutation", async () => {
    const result = await httpClient.mutation(
      api.textSearch.fullTextSearchMutation,
      {
        query: "al pastor",
        cuisine: EXAMPLE_DATA[0].cuisine + "no",
      },
    );
    expect(result).toStrictEqual([]);
  });
});
