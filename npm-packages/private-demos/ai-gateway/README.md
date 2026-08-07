# AI gateway

This internal demo exercises `getAccessToken("ai")` against the local AI gateway
from regular and Node actions.

Start Funrun, Conductor, Usher, and the LLM gateway as described in
[`ops/services/llm-gateway/release.md`](../../../ops/services/llm-gateway/release.md).
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

- Pass `gatewayUrl` to target another gateway.

- The OpenAI SDK accepts the Convex token through its asynchronous `apiKey`
  provider and sends it as a bearer token.

- The Anthropic SDK requires the Node runtime. It sends `apiKey` as `X-Api-Key`,
  so the example obtains the token first and passes it as `authToken`, which
  uses bearer authentication.
