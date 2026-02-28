import { action } from "./_generated/server";
import { context as otelContext, trace } from "@opentelemetry/api";
import { AsyncLocalStorageContextManager } from "@opentelemetry/context-async-hooks";

class NoopSpanExporter {
  export(
    _spans: unknown[],
    resultCallback: (result: { code: number }) => void,
  ) {
    resultCallback({ code: 0 });
  }

  shutdown() {
    return Promise.resolve();
  }

  forceFlush() {
    return Promise.resolve();
  }
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
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

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 0) {
    return (sorted[middle - 1] + sorted[middle]) / 2;
  }
  return sorted[middle];
}

async function runSimpleAwaitLoop(iterations: number): Promise<void> {
  for (let i = 0; i < iterations; i += 1) {
    await Promise.resolve(i);
  }
}

async function runPromiseAllLoop(
  batches: number,
  batchSize: number,
): Promise<void> {
  for (let batch = 0; batch < batches; batch += 1) {
    const promises: Promise<number>[] = [];
    for (let i = 0; i < batchSize; i += 1) {
      promises.push(Promise.resolve(i));
    }
    await Promise.all(promises);
  }
}

async function runMeasured(
  rounds: number,
  fn: () => Promise<void>,
): Promise<{ samplesMs: number[]; medianMs: number }> {
  const now =
    typeof globalThis.performance?.now === "function"
      ? () => globalThis.performance.now()
      : () => Date.now();
  const samplesMs: number[] = [];
  for (let round = 0; round < rounds; round += 1) {
    const started = now();
    await fn();
    samplesMs.push(now() - started);
  }
  return { samplesMs, medianMs: median(samplesMs) };
}

function percentOverBaseline(
  baselineMs: number,
  measuredMs: number,
): number | null {
  if (baselineMs <= 0) {
    return null;
  }
  return ((measuredMs - baselineMs) / baselineMs) * 100;
}

export const benchPromises = action({
  args: {},
  handler: async () => {
    const rounds = 5;
    const iterations = 200000;
    const promiseAllBatches = 500;
    const promiseAllBatchSize = 200;

    const lazyDescriptorBefore = Object.getOwnPropertyDescriptor(
      globalThis,
      "AsyncLocalStorage",
    );
    const asyncLocalStorageMaterializedAtStart =
      typeof lazyDescriptorBefore?.value !== "undefined";

    const baselineSimple = await runMeasured(rounds, async () => {
      await runSimpleAwaitLoop(iterations);
    });

    const baselinePromiseAll = await runMeasured(rounds, async () => {
      await runPromiseAllLoop(promiseAllBatches, promiseAllBatchSize);
    });

    const asyncHooks = await import("node:async_hooks");

    const descriptorAfterImport = Object.getOwnPropertyDescriptor(
      globalThis,
      "AsyncLocalStorage",
    );

    const importOnlySimple = await runMeasured(rounds, async () => {
      await runSimpleAwaitLoop(iterations);
    });

    const importOnlyPromiseAll = await runMeasured(rounds, async () => {
      await runPromiseAllLoop(promiseAllBatches, promiseAllBatchSize);
    });

    const als = new asyncHooks.AsyncLocalStorage<number>();

    const activeAlsSimple = await runMeasured(rounds, async () => {
      for (let i = 0; i < iterations; i += 1) {
        await als.run(i, async () => {
          await Promise.resolve(i);
          if (als.getStore() !== i) {
            throw new Error("ALS context lost in simple await benchmark");
          }
        });
      }
    });

    const activeAlsPromiseAll = await runMeasured(rounds, async () => {
      for (let batch = 0; batch < promiseAllBatches; batch += 1) {
        const promises: Promise<void>[] = [];
        for (let i = 0; i < promiseAllBatchSize; i += 1) {
          const id = batch * promiseAllBatchSize + i;
          promises.push(
            als.run(id, async () => {
              await Promise.resolve(id);
              if (als.getStore() !== id) {
                throw new Error("ALS context lost in Promise.all benchmark");
              }
            }),
          );
        }
        await Promise.all(promises);
      }
    });

    const baselineSimpleUs = (baselineSimple.medianMs / iterations) * 1000;
    const importOnlySimpleUs = (importOnlySimple.medianMs / iterations) * 1000;
    const activeAlsSimpleUs = (activeAlsSimple.medianMs / iterations) * 1000;

    const totalPromiseAllOps = promiseAllBatches * promiseAllBatchSize;
    const baselinePromiseAllUs =
      (baselinePromiseAll.medianMs / totalPromiseAllOps) * 1000;
    const importOnlyPromiseAllUs =
      (importOnlyPromiseAll.medianMs / totalPromiseAllOps) * 1000;
    const activeAlsPromiseAllUs =
      (activeAlsPromiseAll.medianMs / totalPromiseAllOps) * 1000;

    const importOnlySimpleOverhead = percentOverBaseline(
      baselineSimple.medianMs,
      importOnlySimple.medianMs,
    );
    const importOnlyPromiseAllOverhead = percentOverBaseline(
      baselinePromiseAll.medianMs,
      importOnlyPromiseAll.medianMs,
    );

    const restorePerformance = installPerformanceShimIfNeeded();
    let aiSdkGenerateText:
      | {
          rounds: number;
          callsPerRound: number;
          noTelemetry: {
            medianMs: number;
            samplesMs: number[];
            medianUsPerCall: number;
          };
          withTelemetry: {
            medianMs: number;
            samplesMs: number[];
            medianUsPerCall: number;
            overheadVsNoTelemetryPercent: number | null;
          };
        }
      | { skipped: string };

    try {
      const { generateText } = await import("ai");
      const { MockLanguageModelV3 } = await import("ai/test");
      const sdk = await import("@opentelemetry/sdk-trace-base");

      const contextManager = new AsyncLocalStorageContextManager();
      contextManager.enable();
      otelContext.setGlobalContextManager(contextManager);

      const provider = new sdk.BasicTracerProvider();
      provider.addSpanProcessor(
        new sdk.SimpleSpanProcessor(new NoopSpanExporter()),
      );
      provider.register();

      try {
        const tracer = trace.getTracer("convex-ai-sdk-bench");
        const aiRounds = 5;
        const aiCallsPerRound = 100;

        const model = new MockLanguageModelV3({
          modelId: "mock-ai-sdk-bench-model",
          doGenerate: async () => ({
            content: [{ type: "text", text: "ok" }],
            finishReason: { unified: "stop", raw: "stop" },
            usage: {
              inputTokens: {
                total: 3,
                noCache: 3,
                cacheRead: 0,
                cacheWrite: 0,
              },
              outputTokens: { total: 2, text: 2, reasoning: 0 },
            },
            warnings: [],
          }),
        });

        const noTelemetry = await runMeasured(aiRounds, async () => {
          for (let i = 0; i < aiCallsPerRound; i += 1) {
            await generateText({
              model,
              prompt: `bench-no-telemetry-${i}`,
              experimental_telemetry: {
                isEnabled: false,
                tracer,
              },
            });
          }
        });

        const withTelemetry = await runMeasured(aiRounds, async () => {
          for (let i = 0; i < aiCallsPerRound; i += 1) {
            await generateText({
              model,
              prompt: `bench-with-telemetry-${i}`,
              experimental_telemetry: {
                isEnabled: true,
                functionId: "convex-ai-sdk-bench",
                tracer,
              },
            });
          }
        });

        const noTelemetryUsPerCall =
          (noTelemetry.medianMs / aiCallsPerRound) * 1000;
        const withTelemetryUsPerCall =
          (withTelemetry.medianMs / aiCallsPerRound) * 1000;

        aiSdkGenerateText = {
          rounds: aiRounds,
          callsPerRound: aiCallsPerRound,
          noTelemetry: {
            medianMs: noTelemetry.medianMs,
            samplesMs: noTelemetry.samplesMs,
            medianUsPerCall: Number(noTelemetryUsPerCall.toFixed(2)),
          },
          withTelemetry: {
            medianMs: withTelemetry.medianMs,
            samplesMs: withTelemetry.samplesMs,
            medianUsPerCall: Number(withTelemetryUsPerCall.toFixed(2)),
            overheadVsNoTelemetryPercent:
              percentOverBaseline(
                noTelemetry.medianMs,
                withTelemetry.medianMs,
              ) === null
                ? null
                : Number(
                    percentOverBaseline(
                      noTelemetry.medianMs,
                      withTelemetry.medianMs,
                    )!.toFixed(2),
                  ),
          },
        };
      } finally {
        await provider.shutdown();
        otelContext.disable();
      }
    } catch (error) {
      aiSdkGenerateText = {
        skipped: `AI SDK telemetry benchmark unavailable: ${errorMessage(error)}`,
      };
    } finally {
      restorePerformance();
    }

    return {
      lazyInit: {
        beforeImport: {
          hasGetter: typeof lazyDescriptorBefore?.get === "function",
          hasValue: typeof lazyDescriptorBefore?.value !== "undefined",
        },
        materializedAtActionStart: asyncLocalStorageMaterializedAtStart,
        note: asyncLocalStorageMaterializedAtStart
          ? "AsyncLocalStorage was already materialized before this action started (warm isolate state or earlier async_hooks import in this module graph)."
          : "AsyncLocalStorage was still lazy before the first node:async_hooks import in this action.",
        afterImport: {
          hasGetter: typeof descriptorAfterImport?.get === "function",
          hasValue: typeof descriptorAfterImport?.value !== "undefined",
        },
      },
      simpleAwait: {
        rounds,
        iterations,
        baseline: {
          medianMs: baselineSimple.medianMs,
          samplesMs: baselineSimple.samplesMs,
          medianUsPerAwait: Number(baselineSimpleUs.toFixed(2)),
        },
        importOnly: {
          medianMs: importOnlySimple.medianMs,
          samplesMs: importOnlySimple.samplesMs,
          medianUsPerAwait: Number(importOnlySimpleUs.toFixed(2)),
          overheadVsBaselinePercent:
            importOnlySimpleOverhead === null
              ? null
              : Number(importOnlySimpleOverhead.toFixed(2)),
        },
        activeAsyncLocalStorage: {
          medianMs: activeAlsSimple.medianMs,
          samplesMs: activeAlsSimple.samplesMs,
          medianUsPerAwait: Number(activeAlsSimpleUs.toFixed(2)),
        },
      },
      promiseAll: {
        rounds,
        totalPromises: totalPromiseAllOps,
        baseline: {
          medianMs: baselinePromiseAll.medianMs,
          samplesMs: baselinePromiseAll.samplesMs,
          medianUsPerPromise: Number(baselinePromiseAllUs.toFixed(2)),
        },
        importOnly: {
          medianMs: importOnlyPromiseAll.medianMs,
          samplesMs: importOnlyPromiseAll.samplesMs,
          medianUsPerPromise: Number(importOnlyPromiseAllUs.toFixed(2)),
          overheadVsBaselinePercent:
            importOnlyPromiseAllOverhead === null
              ? null
              : Number(importOnlyPromiseAllOverhead.toFixed(2)),
        },
        activeAsyncLocalStorage: {
          medianMs: activeAlsPromiseAll.medianMs,
          samplesMs: activeAlsPromiseAll.samplesMs,
          medianUsPerPromise: Number(activeAlsPromiseAllUs.toFixed(2)),
        },
      },
      aiSdkGenerateText,
    };
  },
});
