#!/usr/bin/env node
// Regenerates the AI Gateway model catalog rendered by
// src/components/AiGatewayModels.tsx.

import { appendFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

// The get-convex/ai-gateway-models-probe project (Convex team `nicolascvx`) proxies the
// gateway's `GET /v1/models` because that endpoint needs a service token this
// workflow does not have.
const MODELS_ENDPOINT = "https://modest-hyena-982.convex.cloud/api/action";

// Display names for the providers we know. A prefix missing here is still
// listed, under its raw prefix, so a new provider shows up in the docs (and in
// the automated PR) instead of silently falling off.
const PROVIDER_NAMES = new Map([
  ["openai", "OpenAI"],
  ["anthropic", "Anthropic"],
  ["google", "Google"],
  ["x-ai", "xAI"],
  ["deepseek", "DeepSeek"],
  ["meta-llama", "Meta"],
  ["mistralai", "Mistral"],
  ["moonshotai", "Moonshot"],
  ["amazon", "Amazon"],
  ["qwen", "Qwen"],
  ["z-ai", "Z.AI"],
  ["perplexity", "Perplexity"],
  ["nvidia", "NVIDIA"],
  ["microsoft", "Microsoft"],
  ["cohere", "Cohere"],
  ["minimax", "MiniMax"],
  ["tencent", "Tencent"],
  ["baidu", "Baidu"],
  ["xiaomi", "Xiaomi"],
  ["bytedance", "ByteDance"],
  ["ibm-granite", "IBM Granite"],
  ["openrouter", "OpenRouter"],
  ["aion-labs", "Aion Labs"],
  ["anthracite-org", "Anthracite"],
  ["arcee-ai", "Arcee AI"],
  ["bytedance-seed", "ByteDance Seed"],
  ["cognitivecomputations", "Cognitive Computations"],
  ["dots-studio", "Dots Studio"],
  ["gryphe", "Gryphe"],
  ["inception", "Inception"],
  ["inclusionai", "Inclusion AI"],
  ["inference-net", "Inference.net"],
  ["kwaipilot", "KwaiPilot"],
  ["liquid", "Liquid AI"],
  ["mancer", "Mancer"],
  ["meituan", "Meituan"],
  ["morph", "Morph"],
  ["nex-agi", "Nex AGI"],
  ["nousresearch", "Nous Research"],
  ["perceptron", "Perceptron"],
  ["poolside", "Poolside"],
  ["prism-ml", "Prism ML"],
  ["rekaai", "Reka AI"],
  ["relace", "Relace"],
  ["sakana", "Sakana AI"],
  ["sao10k", "Sao10k"],
  ["stepfun", "StepFun"],
  ["thedrummer", "TheDrummer"],
  ["thinkingmachines", "Thinking Machines"],
  ["unbiased", "Unbiased"],
  ["undi95", "Undi95"],
  ["upstage", "Upstage"],
  ["writer", "Writer"],
]);

// Prefixes whose models are merged into another prefix's group (e.g. tilde
// variants used by some routing layers, or alternate slugs for the same provider).
const MERGE_INTO = new Map([
  ["~anthropic", "anthropic"],
  ["~deepseek", "deepseek"],
  ["~google", "google"],
  ["~moonshotai", "moonshotai"],
  ["~openai", "openai"],
  ["~x-ai", "x-ai"],
  ["~z-ai", "z-ai"],
  ["meta", "meta-llama"],
]);

const OUTPUT_PATH = fileURLToPath(
  new URL("../src/data/ai-gateway-models.json", import.meta.url),
);

async function fetchModelIds() {
  const response = await fetch(MODELS_ENDPOINT, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path: "models:list", args: {}, format: "json" }),
  });
  if (!response.ok) {
    throw new Error(
      `${MODELS_ENDPOINT} returned ${response.status} ${response.statusText}`,
    );
  }
  const body = await response.json();
  if (body.status !== "success") {
    throw new Error(`models:list failed: ${body.errorMessage}`);
  }
  return body.value;
}

const modelIds = await fetchModelIds();

const byPrefix = new Map();
for (const id of modelIds) {
  const rawPrefix = id.split("/")[0];
  const prefix = MERGE_INTO.get(rawPrefix) ?? rawPrefix;
  if (!byPrefix.has(prefix)) {
    byPrefix.set(prefix, []);
  }
  byPrefix.get(prefix).push(id);
}

const knownPrefixes = [...PROVIDER_NAMES.keys()].filter((prefix) =>
  byPrefix.has(prefix),
);
const unknownPrefixes = [...byPrefix.keys()]
  .filter((prefix) => !PROVIDER_NAMES.has(prefix))
  .sort();
const providers = [...knownPrefixes, ...unknownPrefixes].map((prefix) => ({
  name: PROVIDER_NAMES.get(prefix) ?? prefix,
  models: byPrefix.get(prefix).sort(),
}));

writeFileSync(OUTPUT_PATH, JSON.stringify({ providers }, null, 2) + "\n");

console.error(
  `Wrote ${modelIds.length} models from ${providers.length} providers.`,
);
if (unknownPrefixes.length > 0) {
  console.error(
    `Warning: no display name for ${unknownPrefixes.join(", ")}; add them to PROVIDER_NAMES in ${process.argv[1]}.`,
  );
}
if (process.env.GITHUB_OUTPUT !== undefined) {
  appendFileSync(
    process.env.GITHUB_OUTPUT,
    `unknown_providers=${unknownPrefixes.join(", ")}\n`,
  );
}
