# AI gateway

This internal demo exercises `getServiceToken("ai-gateway")` against the local
AI gateway from regular and Node actions, plus the published-style
`@convex-dev/ai-sdk-provider` path that targets `ai-gateway.convex.dev`.

Start Funrun, Conductor, Usher, and the AI gateway as described in
[`ops/services/ai-gateway/release.md`](../../../ops/services/ai-gateway/release.md).
Then deploy and run the examples:

```shell
just convex-usher dev --once
just convex-usher run fetch:listModels '{}'
just convex-usher run openai:listModels '{}'
just convex-usher run node/fetch:listModels '{}'
just convex-usher run node/openai:listModels '{}'
just convex-usher run node/anthropic:listModels '{}'
just convex-usher run openai:chatCompletion \
  '{"prompt":"Reply with one short sentence."}'
just convex-usher run node/openai:chatCompletion \
  '{"prompt":"Reply with one short sentence."}'
```

Against a reachable `ai-gateway.convex.dev` (or hosts override), run the AI SDK
provider example:

```shell
just convex-usher run node/aiSdkProvider:chatCompletion \
  '{"prompt":"Reply with one short sentence."}'
```

- Pass `gatewayUrl` to target another gateway on the OpenAI / fetch / Anthropic
  examples. The AI SDK provider hard-codes `https://ai-gateway.convex.dev/v1`.

- The OpenAI SDK accepts the Convex token through its asynchronous `apiKey`
  provider and sends it as a bearer token.

- The Anthropic SDK requires the Node runtime. It sends `apiKey` as `X-Api-Key`,
  so the example obtains the token first and passes it as `authToken`, which
  uses bearer authentication.
