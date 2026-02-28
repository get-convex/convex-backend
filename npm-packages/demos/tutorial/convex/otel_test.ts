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

import { trace, SpanStatusCode, SpanKind } from "@opentelemetry/api";

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

export const testOtelSpans = action({
  args: {},
  handler: async () => {
    const results: string[] = [];
    const restorePerformance = installPerformanceShimIfNeeded();
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
      const provider = new TracerProvider();
      provider.addSpanProcessor(new SimpleSpanProcessor(exporter));
      provider.register();

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
              await new Promise((r) => setTimeout(r, Math.random() * 10));

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

      otelContext.disable();
      provider.shutdown();

      return results.join("\n");
    } finally {
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
    const errorMessage = (error: unknown) =>
      error instanceof Error ? error.message : String(error);
    const restorePerformance = installPerformanceShimIfNeeded();
    try {
      const contextManager = new AsyncLocalStorageContextManager();
      contextManager.enable();
      otelContext.setGlobalContextManager(contextManager);

      let provider: any;
      const exporter = new InMemorySpanExporter();

      try {
        const sdk = await import("@opentelemetry/sdk-trace-base");
        provider = new sdk.BasicTracerProvider();
        provider.addSpanProcessor(new sdk.SimpleSpanProcessor(exporter));
        provider.register();
        results.push("PASS: OTel tracer provider registered");
      } catch (e) {
        results.push(
          `FAIL: OTel setup for AI SDK telemetry: ${errorMessage(e)}`,
        );
        otelContext.disable();
        return results.join("\n");
      }

      try {
        const { generateText } = await import("ai");
        const { MockLanguageModelV3 } = await import("ai/test");

        const model = new MockLanguageModelV3({
          modelId: "mock-ai-sdk-model",
          doGenerate: async () => ({
            content: [{ type: "text", text: "hello from mock model" }],
            finishReason: { unified: "stop", raw: "stop" },
            usage: {
              inputTokens: {
                total: 3,
                noCache: 3,
                cacheRead: 0,
                cacheWrite: 0,
              },
              outputTokens: { total: 4, text: 4, reasoning: 0 },
            },
            warnings: [],
          }),
        });

        const tracer = trace.getTracer("convex-ai-sdk-telemetry");

        const generated = await generateText({
          model,
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

        const spanNames = exporter.spans.map((span: any) => String(span?.name));
        const hasTopLevelSpan = spanNames.includes("ai.generateText");
        const hasDoGenerateSpan = spanNames.includes(
          "ai.generateText.doGenerate",
        );
        results.push(
          hasTopLevelSpan && hasDoGenerateSpan
            ? "PASS: AI SDK telemetry spans emitted"
            : `FAIL: Missing expected spans. got=${JSON.stringify(spanNames)}`,
        );

        const hasFunctionId = exporter.spans.some(
          (span: any) =>
            span?.attributes?.["ai.telemetry.functionId"] ===
            "convex-ai-sdk-telemetry-test",
        );
        results.push(
          hasFunctionId
            ? "PASS: functionId telemetry attribute recorded"
            : "FAIL: functionId telemetry attribute missing",
        );

        const hasMetadata = exporter.spans.some(
          (span: any) =>
            span?.attributes?.["ai.telemetry.metadata.experiment"] ===
            "convex-tutorial",
        );
        results.push(
          hasMetadata
            ? "PASS: metadata telemetry attribute recorded"
            : "FAIL: metadata telemetry attribute missing",
        );

        exporter.reset();
        await generateText({
          model,
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

      otelContext.disable();
      if (provider) {
        await provider.shutdown();
      }

      return results.join("\n");
    } finally {
      restorePerformance();
    }
  },
});

import { AsyncLocalStorage } from "async_hooks";

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
