import { api } from "./convex/_generated/api";
import { deploymentUrl } from "./common";
import { ConvexHttpClient } from "convex/browser";

describe("HTTPClient", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  test("Staged db indexes", async () => {
    await expect(
      httpClient.query(api.stagedIndexes.badDbIndex),
    ).rejects.toThrow(
      "Index stagedIndexes.by_name is currently staged and not available to query until it is enabled",
    );
  });

  test("Staged search indexes", async () => {
    await expect(
      httpClient.query(api.stagedIndexes.badSearchIndex),
    ).rejects.toThrow(
      "Index stagedIndexes.search_by_name is currently staged and not available to query until it is enabled",
    );
  });

  test("Staged vector indexes", async () => {
    await expect(
      httpClient.action(api.stagedIndexes.badVectorSearch),
    ).rejects.toThrow(
      "Index stagedIndexes.by_embedding is currently staged and not available to query until it is enabled",
    );
  });
});
