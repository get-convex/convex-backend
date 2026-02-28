import { action } from "./_generated/server";

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

    return {
      lazyInit: {
        beforeImport: {
          hasGetter: typeof lazyDescriptorBefore?.get === "function",
          hasValue: typeof lazyDescriptorBefore?.value !== "undefined",
        },
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
    };
  },
});
