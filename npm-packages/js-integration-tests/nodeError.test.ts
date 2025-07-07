import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api";
import { deploymentUrl } from "./common";

describe("HTTPClient", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  test("Node error gives reasonable error message", async () => {
    await expect(httpClient.action(api.nodeError.default)).rejects.toThrow(
      "uncaughtException: Yikes",
    );
  });
});
