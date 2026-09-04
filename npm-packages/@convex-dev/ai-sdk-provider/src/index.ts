import { createAnthropic } from "@ai-sdk/anthropic";
import { createOpenAI } from "@ai-sdk/openai";
import {
  createOpenAICompatible,
  type MetadataExtractor,
} from "@ai-sdk/openai-compatible";
import {
  defaultSettingsMiddleware,
  wrapEmbeddingModel,
  wrapLanguageModel,
} from "ai";
import { getServiceToken } from "convex/server";

type Provider = ReturnType<typeof createOpenAICompatible>;
type ChatModel = ReturnType<Provider>;
type EmbeddingModel = ReturnType<Provider["embeddingModel"]>;
type LanguageModel = Parameters<typeof wrapLanguageModel>[0]["model"];
type GatewayLanguageModel = ReturnType<typeof wrapLanguageModel>;

const maxEmbeddingsPerCall = 512;
// Official providers require a credential before gatewayFetch replaces it with a deployment JWT.
const placeholderCredential = "convex-gateway";

/**
 * A deployment can set `CONVEX_INTERNAL_AI_GATEWAY_HOST` to reach a different
 * gateway, which is how internal apps use staging.
 */
const productionGatewayHost = "https://ai-gateway.convex.dev";

function gatewayBaseURL(): string {
  return `${process.env.CONVEX_INTERNAL_AI_GATEWAY_HOST || productionGatewayHost}/v1`;
}

async function gatewayFetch(
  input: RequestInfo | URL,
  init?: RequestInit,
): Promise<Response> {
  if (typeof getServiceToken !== "function") {
    throw new Error(
      "@convex-dev/ai-sdk-provider requires convex >= 1.45 with getServiceToken support",
    );
  }
  const token = await getServiceToken("ai-gateway");
  const headers = new Headers(init?.headers);
  // Deployment JWT is the only accepted credential for the hosted gateway.
  headers.set("Authorization", `Bearer ${token}`);
  return globalThis.fetch(input, { ...init, headers });
}

/**
 * The gateway returns the authoritative dollar cost of each request in the
 * OpenAI-compatible `usage` object — `usage.cost` (total USD) plus a
 * `usage.cost_details` breakdown. The AI SDK's usage mapping doesn't carry
 * these, so surface them as provider metadata under the `convexGateway` key:
 *
 *   const { providerMetadata } = await generateText({ model: convexGateway(id), ... });
 *   providerMetadata?.convexGateway?.cost;         // e.g. 3.9e-6 (USD)
 *   providerMetadata?.convexGateway?.costDetails;  // per-part breakdown
 */
type ProviderMetadata = Awaited<
  ReturnType<MetadataExtractor["extractMetadata"]>
>;

function convexGatewayUsageMetadata(usage: unknown): ProviderMetadata {
  if (!usage || typeof usage !== "object") return undefined;
  const u = usage as Record<string, unknown>;
  const meta: Record<string, unknown> = {};
  if (typeof u.cost === "number") meta.cost = u.cost;
  if (u.cost_details && typeof u.cost_details === "object") {
    meta.costDetails = u.cost_details;
  }
  return Object.keys(meta).length > 0
    ? ({ convexGateway: meta } as ProviderMetadata)
    : undefined;
}

const costMetadataExtractor: MetadataExtractor = {
  extractMetadata: async ({ parsedBody }) =>
    convexGatewayUsageMetadata(
      (parsedBody as { usage?: unknown } | undefined)?.usage,
    ),
  createStreamExtractor: () => {
    // Streamed responses deliver usage (incl. cost) on the final chunk.
    let usage: unknown;
    return {
      processChunk(parsedChunk: unknown) {
        const chunk = parsedChunk as { usage?: unknown } | undefined;
        if (chunk?.usage) usage = chunk.usage;
      },
      buildMetadata: () => convexGatewayUsageMetadata(usage),
    };
  },
};

function createGatewayProvider(): Provider {
  return createOpenAICompatible({
    name: "convexGateway",
    baseURL: gatewayBaseURL(),
    fetch: gatewayFetch,
    metadataExtractor: costMetadataExtractor,
    supportsStructuredOutputs: true,
    supportedUrls: () => ({ "image/*": [/^https?:\/\/.*$/] }),
  });
}

function sdkModelId(
  gatewayModelId: string,
  provider: "anthropic" | "openai",
): string {
  const prefix = `${provider}/`;
  const modelId = gatewayModelId.startsWith(prefix)
    ? gatewayModelId.slice(prefix.length)
    : gatewayModelId;
  // OpenRouter uses dots in Anthropic versions; the Anthropic SDK's capability lookup uses hyphens.
  return provider === "anthropic" ? modelId.replaceAll(".", "-") : modelId;
}

function gatewayModelFetch(
  gatewayModelId: string,
  input: RequestInfo | URL,
  init?: RequestInit,
): Promise<Response> {
  const body = JSON.parse(init?.body as string);
  body.model = gatewayModelId;
  return gatewayFetch(input, { ...init, body: JSON.stringify(body) });
}

function gatewayLanguageModel(
  modelId: string,
  model: LanguageModel,
  middleware?: Parameters<typeof wrapLanguageModel>[0]["middleware"],
): GatewayLanguageModel {
  return wrapLanguageModel({
    model,
    modelId,
    // The gateway does not expose the providers' batch APIs.
    middleware: middleware ?? { specificationVersion: "v4" },
  });
}

/**
 * The recommended model interface for text generation through the Convex AI gateway.
 * Use `messages` or `responses` only for endpoint-specific features.
 *
 * `getServiceToken` reuses one token for the current action, so calling this
 * more than once in the same action is fine.
 */
export function convexGateway(modelId: string): ChatModel {
  return createGatewayProvider()(modelId);
}

convexGateway.messages = function (modelId: string): GatewayLanguageModel {
  const provider = createAnthropic({
    name: "convexGateway.messages",
    baseURL: gatewayBaseURL(),
    authToken: placeholderCredential,
    fetch: (input, init) => gatewayModelFetch(modelId, input, init),
  });
  return gatewayLanguageModel(
    modelId,
    provider.messages(sdkModelId(modelId, "anthropic")),
  );
};

convexGateway.responses = function (modelId: string): GatewayLanguageModel {
  const provider = createOpenAI({
    name: "convexGateway.responses",
    baseURL: gatewayBaseURL(),
    apiKey: placeholderCredential,
    fetch: (input, init) => gatewayModelFetch(modelId, input, init),
  });
  return gatewayLanguageModel(
    modelId,
    provider.responses(sdkModelId(modelId, "openai")),
    defaultSettingsMiddleware({
      settings: { providerOptions: { openai: { store: false } } },
    }),
  );
};

convexGateway.embeddingModel = function (modelId: string): EmbeddingModel {
  return wrapEmbeddingModel({
    model: createGatewayProvider().embeddingModel(modelId),
    middleware: {
      specificationVersion: "v4",
      overrideMaxEmbeddingsPerCall: () => maxEmbeddingsPerCall,
    },
  });
};
