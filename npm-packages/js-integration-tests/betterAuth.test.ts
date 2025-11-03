import { ConvexHttpClient } from "convex/browser";
import { makeFunctionReference } from "convex/server";
import { deploymentUrl } from "./common";

describe("betterAuth path blocking", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });

  test("betterAuth/ paths should be blocked and return 'not found' error", async () => {
    // Try to call a function in the betterAuth/ directory
    // This should be blocked by the validation logic and return the same error
    // as if the function doesn't exist
    await expect(
      httpClient.query(
        makeFunctionReference<"query">("betterAuth/testFunction:default"),
        {},
      ),
    ).rejects.toThrow(
      "Could not find public function for 'betterAuth/testFunction'. Did you forget to run `npx convex dev` or `npx convex deploy`?",
    );
  });
});
