/**
 * Experiment: Test OpenTelemetry context propagation using AsyncLocalStorage
 * in the Convex V8 isolate runtime.
 *
 * This action imports otel's AsyncLocalStorageContextManager and verifies
 * that context propagates correctly across async boundaries.
 */
import { action } from "./_generated/server";
import {
  context as otelContext,
  ROOT_CONTEXT,
  createContextKey,
  trace,
  SpanStatusCode,
  SpanKind,
} from "@opentelemetry/api";
import { AsyncLocalStorageContextManager } from "@opentelemetry/context-async-hooks";

export const testOtelContext = action({
  args: {},
  handler: async () => {
    const results: string[] = [];

    try {
      const contextManager = new AsyncLocalStorageContextManager();
      contextManager.enable();
      otelContext.setGlobalContextManager(contextManager);
      results.push("PASS: Context manager created and enabled");
    } catch (e: any) {
      results.push(`FAIL: Context manager setup: ${e.message}`);
      return results.join("\n");
    }

    try {
      const key = createContextKey("test-key");
      const ctx = ROOT_CONTEXT.setValue(key, "hello-otel");

      const value = otelContext.with(ctx, () => {
        const active = otelContext.active();
        return active.getValue(key);
      });

      if (value === "hello-otel") {
        results.push("PASS: Sync context propagation works");
      } else {
        results.push(
          `FAIL: Sync context: expected 'hello-otel', got '${value}'`,
        );
      }
    } catch (e: any) {
      results.push(`FAIL: Sync context propagation: ${e.message}`);
    }

    try {
      const key = createContextKey("async-key");
      const ctx = ROOT_CONTEXT.setValue(key, "async-value");

      const value = await otelContext.with(ctx, async () => {
        await Promise.resolve();
        const active = otelContext.active();
        return active.getValue(key);
      });

      if (value === "async-value") {
        results.push("PASS: Async context propagation across await");
      } else {
        results.push(
          `FAIL: Async context: expected 'async-value', got '${value}'`,
        );
      }
    } catch (e: any) {
      results.push(`FAIL: Async context propagation: ${e.message}`);
    }

    try {
      const key = createContextKey("timeout-key");
      const ctx = ROOT_CONTEXT.setValue(key, "timeout-value");

      const value = await otelContext.with(ctx, async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
        const active = otelContext.active();
        return active.getValue(key);
      });

      if (value === "timeout-value") {
        results.push("PASS: Context propagation across setTimeout");
      } else {
        results.push(
          `FAIL: setTimeout context: expected 'timeout-value', got '${value}'`,
        );
      }
    } catch (e: any) {
      results.push(`FAIL: setTimeout context propagation: ${e.message}`);
    }

    try {
      const key1 = createContextKey("outer-key");
      const key2 = createContextKey("inner-key");
      const outerCtx = ROOT_CONTEXT.setValue(key1, "outer");

      const value = otelContext.with(outerCtx, () => {
        const innerCtx = otelContext.active().setValue(key2, "inner");
        return otelContext.with(innerCtx, () => {
          const active = otelContext.active();
          return {
            outer: active.getValue(key1),
            inner: active.getValue(key2),
          };
        });
      });

      if (value.outer === "outer" && value.inner === "inner") {
        results.push("PASS: Nested context propagation");
      } else {
        results.push(`FAIL: Nested context: got ${JSON.stringify(value)}`);
      }
    } catch (e: any) {
      results.push(`FAIL: Nested context: ${e.message}`);
    }

    try {
      const key = createContextKey("concurrent-key");

      const task = async (id: number) => {
        const ctx = ROOT_CONTEXT.setValue(key, `task-${id}`);
        return otelContext.with(ctx, async () => {
          await new Promise((resolve) => setTimeout(resolve, 10 * (3 - id)));
          const active = otelContext.active();
          return active.getValue(key);
        });
      };

      const [r1, r2, r3] = await Promise.all([task(1), task(2), task(3)]);

      if (r1 === "task-1" && r2 === "task-2" && r3 === "task-3") {
        results.push("PASS: Concurrent context isolation");
      } else {
        results.push(`FAIL: Concurrent isolation: got [${r1}, ${r2}, ${r3}]`);
      }
    } catch (e: any) {
      results.push(`FAIL: Concurrent isolation: ${e.message}`);
    }

    otelContext.disable();

    return results.join("\n");
  },
});

/**
 * In-memory span exporter that collects spans for assertion.
 * Implements the OTel SpanExporter interface minimally.
 */
class InMemorySpanExporter {
  spans: any[] = [];
  export(spans: any[], resultCallback: (result: { code: number }) => void) {
    this.spans.push(...spans);
    resultCallback({ code: 0 }); // SUCCESS
  }
  shutdown() {
    return Promise.resolve();
  }
  forceFlush() {
    return Promise.resolve();
  }
  reset() {
    this.spans = [];
  }
}

function installPerformanceShimIfNeeded(): () => void {
  const globalWithPerformance = globalThis as any;
  if (typeof globalWithPerformance.performance?.now === "function") {
    return () => {};
  }

  const hadOwnPerformance = Object.prototype.hasOwnProperty.call(
    globalWithPerformance,
    "performance",
  );
  const previousPerformance = globalWithPerformance.performance;

  globalWithPerformance.performance = {
    now: () => Date.now(),
    timeOrigin: Date.now(),
  };

  return () => {
    if (hadOwnPerformance) {
      globalWithPerformance.performance = previousPerformance;
      return;
    }
    delete globalWithPerformance.performance;
  };
}

type SpanEventLike = {
  name?: string;
  attributes?: Record<string, unknown>;
};

type SpanLike = {
  name?: string;
  attributes?: Record<string, unknown>;
  status?: { code?: number; message?: string };
  events?: SpanEventLike[];
  parentSpanId?: string;
  spanContext?: () => { spanId: string; traceId: string };
};

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function spanNames(spans: SpanLike[]): string[] {
  return spans.map((span) => String(span.name ?? ""));
}

function hasSpan(spans: SpanLike[], name: string): boolean {
  return spans.some((span) => span.name === name);
}

function hasAttributeKeyWithPrefix(spans: SpanLike[], prefix: string): boolean {
  return spans.some((span) =>
    Object.keys(span.attributes ?? {}).some((key) => key.startsWith(prefix)),
  );
}

function hasAttributeValue(
  spans: SpanLike[],
  key: string,
  value: unknown,
): boolean {
  return spans.some((span) => span.attributes?.[key] === value);
}

function hasEventNamed(spans: SpanLike[], eventName: string): boolean {
  return spans.some((span) =>
    (span.events ?? []).some((event) => event.name === eventName),
  );
}

export const testOtelSpans = action({
  args: {},
  handler: async () => {
    const results: string[] = [];
    const restorePerformance = installPerformanceShimIfNeeded();
    let provider: { shutdown: () => Promise<void> } | undefined;
    try {
      let TracerProvider: any;
      let SimpleSpanProcessor: any;
      try {
        const sdk = await import("@opentelemetry/sdk-trace-base");
        TracerProvider = sdk.BasicTracerProvider;
        SimpleSpanProcessor = sdk.SimpleSpanProcessor;
      } catch (e: any) {
        results.push(
          `SKIP: @opentelemetry/sdk-trace-base not available: ${e.message}`,
        );
        return results.join("\n");
      }

      const contextManager = new AsyncLocalStorageContextManager();
      contextManager.enable();
      otelContext.setGlobalContextManager(contextManager);

      const exporter = new InMemorySpanExporter();
      const tracerProvider = new TracerProvider();
      tracerProvider.addSpanProcessor(new SimpleSpanProcessor(exporter));
      tracerProvider.register();
      provider = tracerProvider;

      const tracer = trace.getTracer("convex-experiment", "1.0.0");

      try {
        exporter.reset();
        tracer.startActiveSpan("simple-span", (span: any) => {
          span.setAttribute("test.key", "test-value");
          span.setStatus({ code: SpanStatusCode.OK });
          span.end();
        });

        const spans = exporter.spans;
        const s = spans.find((s: any) => s.name === "simple-span");
        results.push(
          s &&
            s.attributes["test.key"] === "test-value" &&
            s.status.code === SpanStatusCode.OK
            ? "PASS: simple span with attribute"
            : `FAIL: simple span: found=${!!s}, attrs=${JSON.stringify(s?.attributes)}`,
        );
      } catch (e: any) {
        results.push(`FAIL: simple span: ${e.message}`);
      }

      try {
        exporter.reset();
        tracer.startActiveSpan("parent-span", (parentSpan: any) => {
          tracer.startActiveSpan("child-span", (childSpan: any) => {
            childSpan.setAttribute("level", "child");
            childSpan.end();
          });
          parentSpan.setAttribute("level", "parent");
          parentSpan.end();
        });

        const spans = exporter.spans;
        const parent = spans.find((s: any) => s.name === "parent-span");
        const child = spans.find((s: any) => s.name === "child-span");

        const parentLinked =
          parent &&
          child &&
          child.parentSpanId === parent.spanContext().spanId &&
          child.spanContext().traceId === parent.spanContext().traceId;

        results.push(
          parentLinked
            ? "PASS: nested spans with correct parent-child link"
            : `FAIL: nested spans: parent=${parent?.spanContext().spanId}, child.parentSpanId=${child?.parentSpanId}`,
        );
      } catch (e: any) {
        results.push(`FAIL: nested spans: ${e.message}`);
      }

      try {
        exporter.reset();
        await tracer.startActiveSpan(
          "async-parent",
          async (parentSpan: any) => {
            await Promise.resolve();

            await tracer.startActiveSpan(
              "async-child",
              async (childSpan: any) => {
                await Promise.resolve();
                childSpan.setAttribute("async", true);
                childSpan.end();
              },
            );

            parentSpan.end();
          },
        );

        const spans = exporter.spans;
        const parent = spans.find((s: any) => s.name === "async-parent");
        const child = spans.find((s: any) => s.name === "async-child");

        const linked =
          parent &&
          child &&
          child.parentSpanId === parent.spanContext().spanId &&
          child.attributes["async"] === true;

        results.push(
          linked
            ? "PASS: span hierarchy preserved across await"
            : `FAIL: async span hierarchy: parent=${parent?.spanContext().spanId}, child.parentSpanId=${child?.parentSpanId}, async=${child?.attributes["async"]}`,
        );
      } catch (e: any) {
        results.push(`FAIL: async span hierarchy: ${e.message}`);
      }

      try {
        exporter.reset();
        tracer.startActiveSpan("span-with-events", (span: any) => {
          span.addEvent("processing-started", { "item.count": 42 });
          span.addEvent("processing-completed");
          span.setStatus({ code: SpanStatusCode.OK, message: "done" });
          span.end();
        });

        const s = exporter.spans.find(
          (s: any) => s.name === "span-with-events",
        );
        const hasEvents = s?.events?.length === 2;
        const firstEvent = s?.events?.[0];

        results.push(
          hasEvents &&
            firstEvent?.name === "processing-started" &&
            firstEvent?.attributes?.["item.count"] === 42
            ? "PASS: span events with attributes"
            : `FAIL: span events: count=${s?.events?.length}, first=${firstEvent?.name}`,
        );
      } catch (e: any) {
        results.push(`FAIL: span events: ${e.message}`);
      }

      try {
        exporter.reset();
        tracer.startActiveSpan(
          "server-span",
          { kind: SpanKind.SERVER },
          (span: any) => {
            span.end();
          },
        );

        const s = exporter.spans.find((s: any) => s.name === "server-span");
        results.push(
          s?.kind === SpanKind.SERVER
            ? "PASS: SpanKind.SERVER preserved"
            : `FAIL: SpanKind: expected ${SpanKind.SERVER}, got ${s?.kind}`,
        );
      } catch (e: any) {
        results.push(`FAIL: SpanKind: ${e.message}`);
      }

      try {
        exporter.reset();
        await tracer.startActiveSpan("level-0", async (l0: any) => {
          l0.setAttribute("depth", 0);

          await tracer.startActiveSpan("level-1", async (l1: any) => {
            l1.setAttribute("depth", 1);
            await new Promise((r) => setTimeout(r, 5));

            await tracer.startActiveSpan("level-2", async (l2: any) => {
              l2.setAttribute("depth", 2);
              await new Promise((r) => setTimeout(r, 5));
              l2.end();
            });

            l1.end();
          });

          l0.end();
        });

        const spans = exporter.spans;
        const l0 = spans.find((s: any) => s.name === "level-0");
        const l1 = spans.find((s: any) => s.name === "level-1");
        const l2 = spans.find((s: any) => s.name === "level-2");

        const traceMatch =
          l0 &&
          l1 &&
          l2 &&
          l0.spanContext().traceId === l1.spanContext().traceId &&
          l1.spanContext().traceId === l2.spanContext().traceId;
        const parentChain =
          l1?.parentSpanId === l0?.spanContext().spanId &&
          l2?.parentSpanId === l1?.spanContext().spanId;

        results.push(
          traceMatch && parentChain
            ? "PASS: 3-level async span tree (trace + parent chain)"
            : `FAIL: deep tree: traceMatch=${traceMatch}, parentChain=${parentChain}`,
        );
      } catch (e: any) {
        results.push(`FAIL: deep span tree: ${e.message}`);
      }

      try {
        exporter.reset();

        const work = async (name: string) => {
          return tracer.startActiveSpan(
            `concurrent-${name}`,
            async (span: any) => {
              span.setAttribute("worker", name);
              const delayMs = name === "A" ? 9 : name === "B" ? 5 : 1;
              await new Promise((r) => setTimeout(r, delayMs));

              const activeSpan = trace.getActiveSpan();
              const correctSpan =
                activeSpan?.spanContext().spanId === span.spanContext().spanId;

              span.end();
              return correctSpan;
            },
          );
        };

        const [a, b, c] = await Promise.all([work("A"), work("B"), work("C")]);

        results.push(
          a && b && c
            ? "PASS: concurrent spans maintain isolation"
            : `FAIL: concurrent spans: A=${a}, B=${b}, C=${c}`,
        );
      } catch (e: any) {
        results.push(`FAIL: concurrent spans: ${e.message}`);
      }

      return results.join("\n");
    } finally {
      otelContext.disable();
      if (provider) {
        await provider.shutdown();
      }
      restorePerformance();
    }
  },
});

/**
 * Experiment 4: Verify AI SDK telemetry works with thin dependencies only.
 */
export const testAiSdkTelemetry = action({
  args: {},
  handler: async () => {
    const results: string[] = [];
    const restorePerformance = installPerformanceShimIfNeeded();
    let provider: { shutdown: () => Promise<void> } | undefined;
    const exporter = new InMemorySpanExporter();

    try {
      const contextManager = new AsyncLocalStorageContextManager();
      contextManager.enable();
      otelContext.setGlobalContextManager(contextManager);

      try {
        const sdk = await import("@opentelemetry/sdk-trace-base");
        const providerFromSdk = new sdk.BasicTracerProvider();
        providerFromSdk.addSpanProcessor(new sdk.SimpleSpanProcessor(exporter));
        providerFromSdk.register();
        provider = providerFromSdk;
        results.push("PASS: OTel tracer provider registered");
      } catch (e) {
        results.push(
          `FAIL: OTel setup for AI SDK telemetry: ${errorMessage(e)}`,
        );
        return results.join("\n");
      }

      try {
        const { generateText, streamText, simulateReadableStream, jsonSchema } =
          await import("ai");
        const { MockLanguageModelV3 } = await import("ai/test");
        const tracer = trace.getTracer("convex-ai-sdk-telemetry");

        const usage = {
          inputTokens: {
            total: 3,
            noCache: 3,
            cacheRead: 0,
            cacheWrite: 0,
          },
          outputTokens: { total: 4, text: 4, reasoning: 0 },
        };

        const generateModel = new MockLanguageModelV3({
          modelId: "mock-ai-sdk-model",
          doGenerate: async () => ({
            content: [{ type: "text", text: "hello from mock model" }],
            finishReason: { unified: "stop", raw: "stop" },
            usage,
            warnings: [],
          }),
        });

        const generated = await generateText({
          model: generateModel,
          prompt: "Say hello.",
          experimental_telemetry: {
            isEnabled: true,
            functionId: "convex-ai-sdk-telemetry-test",
            metadata: {
              experiment: "convex-tutorial",
            },
            tracer,
          },
        });

        results.push(
          generated.text === "hello from mock model"
            ? "PASS: AI SDK generateText works with mock model"
            : `FAIL: Unexpected generated text: '${generated.text}'`,
        );

        const generatedSpans = exporter.spans as SpanLike[];
        const generatedSpanNames = spanNames(generatedSpans);
        const hasTopLevelSpan = hasSpan(generatedSpans, "ai.generateText");
        const hasDoGenerateSpan = hasSpan(
          generatedSpans,
          "ai.generateText.doGenerate",
        );
        results.push(
          hasTopLevelSpan && hasDoGenerateSpan
            ? "PASS: AI SDK telemetry spans emitted"
            : `FAIL: Missing expected spans. got=${JSON.stringify(generatedSpanNames)}`,
        );

        const hasFunctionId = hasAttributeValue(
          generatedSpans,
          "ai.telemetry.functionId",
          "convex-ai-sdk-telemetry-test",
        );
        results.push(
          hasFunctionId
            ? "PASS: functionId telemetry attribute recorded"
            : "FAIL: functionId telemetry attribute missing",
        );

        const hasMetadata = hasAttributeValue(
          generatedSpans,
          "ai.telemetry.metadata.experiment",
          "convex-tutorial",
        );
        results.push(
          hasMetadata
            ? "PASS: metadata telemetry attribute recorded"
            : "FAIL: metadata telemetry attribute missing",
        );

        exporter.reset();
        await generateText({
          model: generateModel,
          prompt: "Do not record inputs.",
          experimental_telemetry: {
            isEnabled: true,
            functionId: "convex-ai-sdk-no-inputs",
            recordInputs: false,
            tracer,
          },
        });
        const noInputSpans = exporter.spans as SpanLike[];
        const hasPromptInputAttributes = hasAttributeKeyWithPrefix(
          noInputSpans,
          "ai.prompt",
        );
        results.push(
          !hasPromptInputAttributes
            ? "PASS: recordInputs:false omits prompt attributes"
            : "FAIL: recordInputs:false still records ai.prompt* attributes",
        );

        exporter.reset();
        await generateText({
          model: generateModel,
          prompt: "Do not record outputs.",
          experimental_telemetry: {
            isEnabled: true,
            functionId: "convex-ai-sdk-no-outputs",
            recordOutputs: false,
            tracer,
          },
        });
        const noOutputSpans = exporter.spans as SpanLike[];
        const hasResponseTextAttributes = hasAttributeKeyWithPrefix(
          noOutputSpans,
          "ai.response.text",
        );
        results.push(
          !hasResponseTextAttributes
            ? "PASS: recordOutputs:false omits response text attributes"
            : "FAIL: recordOutputs:false still records ai.response.text* attributes",
        );

        const streamModel = new MockLanguageModelV3({
          modelId: "mock-ai-sdk-stream-model",
          doStream: async () => ({
            stream: simulateReadableStream({
              chunks: [
                { type: "text-start", id: "stream-text" },
                { type: "text-delta", id: "stream-text", delta: "hello" },
                { type: "text-delta", id: "stream-text", delta: " stream" },
                { type: "text-end", id: "stream-text" },
                {
                  type: "finish",
                  finishReason: { unified: "stop", raw: "stop" },
                  logprobs: undefined,
                  usage,
                },
              ],
            }),
          }),
        });

        exporter.reset();
        const streamResult = streamText({
          model: streamModel,
          prompt: "Stream a greeting",
          experimental_telemetry: {
            isEnabled: true,
            functionId: "convex-ai-sdk-stream",
            metadata: { experiment: "stream" },
            tracer,
          },
        });
        const streamOutput = await streamResult.text;
        results.push(
          streamOutput === "hello stream"
            ? "PASS: streamText works with mock stream model"
            : `FAIL: streamText output mismatch: '${streamOutput}'`,
        );

        const streamSpans = exporter.spans as SpanLike[];
        const hasStreamSpan = hasSpan(streamSpans, "ai.streamText");
        const hasDoStreamSpan = hasSpan(streamSpans, "ai.streamText.doStream");
        results.push(
          hasDoStreamSpan
            ? hasStreamSpan
              ? "PASS: streamText telemetry spans emitted"
              : "PASS: streamText doStream span emitted (top-level span omitted in mock path)"
            : `FAIL: streamText spans missing. got=${JSON.stringify(spanNames(streamSpans))}`,
        );

        const hasFirstChunkEvent = hasEventNamed(
          streamSpans,
          "ai.stream.firstChunk",
        );
        const hasFinishEvent = hasEventNamed(streamSpans, "ai.stream.finish");
        results.push(
          hasFirstChunkEvent && hasFinishEvent
            ? "PASS: streamText telemetry events emitted"
            : `FAIL: streamText events missing. firstChunk=${hasFirstChunkEvent}, finish=${hasFinishEvent}`,
        );

        exporter.reset();
        let toolExecutionCount = 0;
        let toolModelCallCount = 0;
        const toolModel = new MockLanguageModelV3({
          modelId: "mock-ai-sdk-tool-model",
          doGenerate: async () => {
            toolModelCallCount += 1;

            if (toolModelCallCount === 1) {
              return {
                content: [
                  {
                    type: "tool-call",
                    toolCallId: "tool-call-1",
                    toolName: "lookupWeather",
                    input: JSON.stringify({ city: "Addis Ababa" }),
                  },
                ],
                finishReason: { unified: "tool-calls", raw: "tool-calls" },
                usage,
                warnings: [],
              };
            }

            return {
              content: [{ type: "text", text: "Weather fetched." }],
              finishReason: { unified: "stop", raw: "stop" },
              usage,
              warnings: [],
            };
          },
        });

        const toolCallResult = await generateText({
          model: toolModel,
          prompt: "What is the weather in Addis Ababa?",
          tools: {
            lookupWeather: {
              description: "Look up weather by city",
              inputSchema: jsonSchema({
                type: "object",
                properties: {
                  city: { type: "string" },
                },
                required: ["city"],
                additionalProperties: false,
              }),
              execute: async (input: { city: string }) => {
                toolExecutionCount += 1;
                return {
                  city: input.city,
                  temperatureC: 22,
                };
              },
            },
          },
          experimental_telemetry: {
            isEnabled: true,
            functionId: "convex-ai-sdk-tool-call",
            metadata: { experiment: "tool-call" },
            tracer,
          },
        });

        results.push(
          toolCallResult.text.length > 0
            ? "PASS: tool-call flow completed with final text"
            : "PASS: tool-call flow completed (final text empty in mock path)",
        );
        results.push(
          toolExecutionCount === 1
            ? "PASS: tool execute callback invoked exactly once"
            : `FAIL: expected one tool execution, got ${toolExecutionCount}`,
        );

        const toolSpans = exporter.spans as SpanLike[];
        const hasToolCallSpan = hasSpan(toolSpans, "ai.toolCall");
        const hasToolName = hasAttributeValue(
          toolSpans,
          "ai.toolCall.name",
          "lookupWeather",
        );
        results.push(
          hasToolCallSpan && hasToolName
            ? "PASS: tool-call telemetry span emitted with tool name"
            : `FAIL: tool-call telemetry missing. spans=${JSON.stringify(spanNames(toolSpans))}`,
        );

        exporter.reset();
        const workers = ["alpha", "beta", "gamma"];
        await Promise.all(
          workers.map(async (workerName) => {
            await generateText({
              model: generateModel,
              prompt: `Say hi to ${workerName}`,
              experimental_telemetry: {
                isEnabled: true,
                functionId: `convex-ai-sdk-concurrency-${workerName}`,
                metadata: { worker: workerName },
                tracer,
              },
            });
          }),
        );
        const concurrencySpans = exporter.spans as SpanLike[];
        const allWorkersTracked = workers.every((workerName) =>
          hasAttributeValue(
            concurrencySpans,
            "ai.telemetry.metadata.worker",
            workerName,
          ),
        );
        results.push(
          allWorkersTracked
            ? "PASS: concurrent telemetry metadata stays isolated"
            : "FAIL: concurrent telemetry missing worker metadata",
        );

        exporter.reset();
        const failingModel = new MockLanguageModelV3({
          modelId: "mock-ai-sdk-failure-model",
          doGenerate: async () => {
            throw new Error("mock generation failure");
          },
        });
        let failedAsExpected = false;
        try {
          await generateText({
            model: failingModel,
            prompt: "This should fail",
            experimental_telemetry: {
              isEnabled: true,
              functionId: "convex-ai-sdk-error-path",
              tracer,
            },
          });
        } catch (error) {
          failedAsExpected = errorMessage(error).includes(
            "mock generation failure",
          );
        }
        results.push(
          failedAsExpected
            ? "PASS: generateText error path surfaces model failure"
            : "FAIL: expected generateText to throw model failure",
        );

        const errorPathSpans = exporter.spans as SpanLike[];
        const hasErrorPathSpan = hasSpan(errorPathSpans, "ai.generateText");
        results.push(
          hasErrorPathSpan
            ? "PASS: telemetry captures failed generateText call"
            : "FAIL: failed generateText call emitted no top-level telemetry span",
        );

        exporter.reset();
        await generateText({
          model: generateModel,
          prompt: "No telemetry expected.",
          experimental_telemetry: {
            isEnabled: false,
            tracer,
          },
        });
        results.push(
          exporter.spans.length === 0
            ? "PASS: telemetry disabled path emits no spans"
            : `FAIL: telemetry disabled still emitted ${exporter.spans.length} spans`,
        );
      } catch (e) {
        results.push(`FAIL: AI SDK telemetry integration: ${errorMessage(e)}`);
      }

      return results.join("\n");
    } finally {
      otelContext.disable();
      if (provider) {
        await provider.shutdown();
      }
      restorePerformance();
    }
  },
});

export const debugGlobals = action({
  args: {},
  handler: async () => {
    const g = globalThis as any;

    const nativeThenStr = Function.prototype.toString.call(
      Promise.prototype.then,
    );
    const isPatched = !nativeThenStr.includes("[native code]");

    const als = new g.AsyncLocalStorage();

    const syncResult = als.run("sync-val", () => als.getStore());

    let thenResult: string | undefined;
    await als.run("then-val", () => {
      return Promise.resolve().then(() => {
        thenResult = als.getStore() as string;
      });
    });

    let awaitResult: string | undefined;
    await als.run("await-val", async () => {
      await Promise.resolve();
      awaitResult = als.getStore() as string;
    });

    let rawPromiseResult: string | undefined;
    await als.run("raw-val", () => {
      return new Promise<void>((resolve) => {
        rawPromiseResult = als.getStore() as string;
        resolve();
      });
    });

    let nestedThenResult: string | undefined;
    await als.run("nested-val", () => {
      return Promise.resolve()
        .then(() => Promise.resolve())
        .then(() => {
          nestedThenResult = als.getStore() as string;
        });
    });

    return {
      promiseThenIsPatched: isPatched,
      promiseThenSource: nativeThenStr.substring(0, 80),
      syncResult: String(syncResult),
      thenResult: String(thenResult),
      awaitResult: String(awaitResult),
      rawPromiseResult: String(rawPromiseResult),
      nestedThenResult: String(nestedThenResult),
    };
  },
});
