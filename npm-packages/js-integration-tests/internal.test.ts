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

  test("Cannot call internal function directly", async () => {
    const messageId = await httpClient.mutation(api.internal.sendMessage, {
      body: "hello",
      channel: "test",
    });
    // @ts-expect-error -- should not compile, internal function
    const result = httpClient.mutation(api.internal.update, {
      messageId,
      body: "hello",
      secondsLeft: 4,
    });
    await expect(result).rejects.toThrow(
      "Could not find public function for 'internal:update'. Did you forget to run `npx convex dev` or `npx convex deploy`?",
    );
  });

  test("Can call internal function indirectly", async () => {
    const messageId = await httpClient.mutation(
      api.internal.sendExpiringMessage,
      {
        body: "hello",
        channel: "test",
      },
    );
    let message = await httpClient.query(api.internal.getMessage, {
      messageId,
    });
    expect(message).not.toBeNull();
    // Wait for the scheduled function to be called and delete the message
    await new Promise((resolve) => setTimeout(resolve, 1500));
    message = await httpClient.query(api.internal.getMessage, { messageId });
    expect(message).toBeNull();
  });
});
