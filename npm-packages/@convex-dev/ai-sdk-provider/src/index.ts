import { createAnthropic } from "@ai-sdk/anthropic";
import { createOpenAI } from "@ai-sdk/openai";
import {
  createOpenAICompatible,
  type MetadataExtractor,
} from "@ai-sdk/openai-compatible";
import {
  defaultSettingsMiddleware,
  type JSONValue,
  type ProviderMetadata,
  wrapEmbeddingModel,
  wrapLanguageModel,
} from "ai";
import { getServiceToken } from "convex/server";

type Provider = ReturnType<typeof createOpenAICompatible>;
type ChatModel = ReturnType<Provider>;
type EmbeddingModel = ReturnType<Provider["embeddingModel"]>;
type LanguageModel = Parameters<typeof wrapLanguageModel>[0]["model"];
type GatewayLanguageModel = ReturnType<typeof wrapLanguageModel>;

export type DecisionsContent = string | Record<string, unknown> | unknown[];

export type DecisionsChoiceQuestion = {
  type: "choice";
  instructions: DecisionsContent;
  criteria: Record<string, DecisionsContent | null>;
};

export type DecisionsScoreQuestion = {
  type: "score";
  instructions: DecisionsContent;
  criteria: DecisionsContent[];
};

export type DecisionsNoulQuestion = {
  type: "noul";
  instructions: DecisionsContent;
  criteria?: {
    true: DecisionsContent;
    false: DecisionsContent;
  };
};

export type DecisionsQuestion =
  | DecisionsChoiceQuestion
  | DecisionsScoreQuestion
  | DecisionsNoulQuestion;

export type DecisionsQuestions = Record<string, DecisionsQuestion>;

export type DecisionsRequest<
  Questions extends DecisionsQuestions = DecisionsQuestions,
> = {
  model: string;
  state: DecisionsContent;
  questions: Questions;
};

export type DecisionsChoiceAnswer = {
  type: "choice";
  choice: string;
  confidence?: number;
  probabilities?: Record<string, number>;
};

export type DecisionsScoreAnswer = {
  type: "score";
  score: number;
  confidence?: number;
  probabilities?: Record<string, number>;
  legend?: Record<string, DecisionsContent>;
};

export type DecisionsNoulAnswer = {
  type: "noul";
  noul: number;
};

export type DecisionsAnswer =
  | DecisionsChoiceAnswer
  | DecisionsScoreAnswer
  | DecisionsNoulAnswer;

export type DecisionsAnswerForQuestion<Question extends DecisionsQuestion> =
  Question extends DecisionsChoiceQuestion
    ? DecisionsChoiceAnswer
    : Question extends DecisionsScoreQuestion
      ? DecisionsScoreAnswer
      : DecisionsNoulAnswer;

export type DecisionsResponse<
  Questions extends DecisionsQuestions = DecisionsQuestions,
> = {
  id: string;
  model: string;
  answers: {
    [Key in keyof Questions]: DecisionsAnswerForQuestion<Questions[Key]>;
  };
  usage: {
    input_tokens: number;
    output_tokens: number;
    cost?: number;
  };
};

export type DecisionsRequestOptions = {
  signal?: AbortSignal;
};

export class ConvexGatewayError extends Error {
  readonly code: "http_error" | "invalid_response";
  readonly status: number;
  readonly body: unknown;

  constructor(
    message: string,
    code: "http_error" | "invalid_response",
    status: number,
    body: unknown,
  ) {
    super(message);
    this.name = "ConvexGatewayError";
    this.code = code;
    this.status = status;
    this.body = body;
  }
}

const maxEmbeddingsPerCall = 512;
// Official providers require a credential before gatewayFetch replaces it with a deployment JWT.
const placeholderCredential = "convex-gateway";

/**
 * A deployment can set `CONVEX_INTERNAL_AI_GATEWAY_HOST` to reach a different
 * gateway, which is how internal apps use staging.
 */
const productionGatewayHost = "https://ai-gateway.convex.dev";

function gatewayBaseURL(version: "v1" | "alpha" = "v1"): string {
  return `${process.env.CONVEX_INTERNAL_AI_GATEWAY_HOST || productionGatewayHost}/${version}`;
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

// The SDK's standard usage mapping omits the gateway's dollar costs.
function convexGatewayUsageMetadata(
  usage: unknown,
): ProviderMetadata | undefined {
  if (!usage || typeof usage !== "object") return undefined;
  const { cost, cost_details } = usage as Record<string, JSONValue>;
  const metadata: ProviderMetadata[string] = {};
  if (typeof cost === "number") metadata.cost = cost;
  if (cost_details && typeof cost_details === "object") {
    metadata.costDetails = cost_details;
  }
  return Object.keys(metadata).length > 0
    ? { convexGateway: metadata }
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function invalidDecisionsResponse(
  response: Response,
  body: unknown,
): ConvexGatewayError {
  return new ConvexGatewayError(
    "The Convex AI Gateway returned an invalid Decisions response",
    "invalid_response",
    response.status,
    body,
  );
}

function isNumberRecord(value: unknown): value is Record<string, number> {
  return (
    isRecord(value) &&
    Object.values(value).every((number) => isFiniteNumber(number))
  );
}

function isDecisionAnswer(answer: unknown): answer is DecisionsAnswer {
  if (!isRecord(answer) || typeof answer.type !== "string") {
    return false;
  }
  if (
    (answer.confidence !== undefined && !isFiniteNumber(answer.confidence)) ||
    (answer.probabilities !== undefined &&
      !isNumberRecord(answer.probabilities))
  ) {
    return false;
  }
  switch (answer.type) {
    case "choice":
      return typeof answer.choice === "string";
    case "score":
      return (
        isFiniteNumber(answer.score) &&
        (answer.legend === undefined || isRecord(answer.legend))
      );
    case "noul":
      return isFiniteNumber(answer.noul);
    default:
      return false;
  }
}

function isDecisionsResponse<Questions extends DecisionsQuestions>(
  body: unknown,
  questions: Questions,
): body is DecisionsResponse<Questions> {
  if (
    !isRecord(body) ||
    typeof body.id !== "string" ||
    typeof body.model !== "string" ||
    !isRecord(body.answers) ||
    !isRecord(body.usage)
  ) {
    return false;
  }
  const answers = body.answers;
  const usage = body.usage;
  return (
    Object.values(answers).every(isDecisionAnswer) &&
    Object.entries(questions).every(
      ([key, question]) =>
        isRecord(answers[key]) && answers[key].type === question.type,
    ) &&
    Number.isSafeInteger(usage.input_tokens) &&
    (usage.input_tokens as number) >= 0 &&
    Number.isSafeInteger(usage.output_tokens) &&
    (usage.output_tokens as number) >= 0 &&
    (usage.cost === undefined ||
      (isFiniteNumber(usage.cost) && usage.cost >= 0))
  );
}

function gatewayErrorMessage(status: number, body: unknown): string {
  if (isRecord(body)) {
    const error = body.error;
    if (isRecord(error) && typeof error.message === "string") {
      return error.message;
    }
  }
  return `The Convex AI Gateway request failed with status ${status}`;
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

/**
 * Answer typed questions about the supplied state through the alpha Decisions API.
 * Returns answers directly. Call this method from an action.
 * The request and response format may change during alpha.
 */
convexGateway.decisions = async function <Questions extends DecisionsQuestions>(
  request: DecisionsRequest<Questions>,
  options: DecisionsRequestOptions = {},
): Promise<DecisionsResponse<Questions>> {
  const response = await gatewayFetch(`${gatewayBaseURL("alpha")}/decisions`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
    signal: options.signal,
  });
  const text = await response.text();
  let body: unknown = text;
  try {
    body = JSON.parse(text);
  } catch {
    if (response.ok) {
      throw invalidDecisionsResponse(response, body);
    }
  }
  if (!response.ok) {
    throw new ConvexGatewayError(
      gatewayErrorMessage(response.status, body),
      "http_error",
      response.status,
      body,
    );
  }
  if (!isDecisionsResponse(body, request.questions)) {
    throw invalidDecisionsResponse(response, body);
  }
  return body;
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
