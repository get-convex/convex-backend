import { createOpenAICompatible } from "@ai-sdk/openai-compatible";
import { getServiceToken } from "convex/server";

type ChatModel = ReturnType<ReturnType<typeof createOpenAICompatible>>;

const provider = createOpenAICompatible({
  name: "convexGateway",
  baseURL: "https://ai-gateway.convex.dev/v1",
  fetch: async (input, init) => {
    if (typeof getServiceToken !== "function") {
      throw new Error(
        "@convex-dev/ai-sdk-provider requires convex >= 1.43 with getServiceToken support",
      );
    }
    const token = await getServiceToken("ai-gateway");
    const headers = new Headers(init?.headers);
    // Deployment JWT is the only accepted credential for the hosted gateway.
    headers.set("Authorization", `Bearer ${token}`);
    return await globalThis.fetch(input, { ...init, headers });
  },
});

/**
 * Chat model for the hosted Convex AI gateway.
 * `getServiceToken` reuses one token for the current action, so calling this
 * more than once in the same action is fine.
 */
export function convexGateway(modelId: string): ChatModel {
  return provider(modelId);
}
