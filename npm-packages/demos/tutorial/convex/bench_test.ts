/**
 * Experiment 3: Measure overhead of async context propagation.
 *
 * This action creates many promises to measure per-promise overhead
 * of AsyncLocalStorage context propagation in the runtime.
 */
import { action } from "./_generated/server";

export const benchPromises = action({
  args: {},
  handler: async () => {
    const iterations = 10000;

    // Benchmark 1: Simple promise chains
    const start1 = Date.now();
    for (let i = 0; i < iterations; i++) {
      await Promise.resolve(i);
    }
    const elapsed1 = Date.now() - start1;

    // Benchmark 2: Promise.all with many promises
    const start2 = Date.now();
    for (let j = 0; j < 100; j++) {
      const promises = [];
      for (let i = 0; i < 100; i++) {
        promises.push(Promise.resolve(i));
      }
      await Promise.all(promises);
    }
    const elapsed2 = Date.now() - start2;

    // Benchmark 3: Nested async functions
    const start3 = Date.now();
    async function nested(depth: number): Promise<number> {
      if (depth <= 0) return 0;
      return 1 + (await nested(depth - 1));
    }
    for (let i = 0; i < 100; i++) {
      await nested(100);
    }
    const elapsed3 = Date.now() - start3;

    return {
      simpleAwait: `${iterations} awaits in ${elapsed1}ms (${((elapsed1 / iterations) * 1000).toFixed(1)}μs/await)`,
      promiseAll: `${100 * 100} promises in ${elapsed2}ms (${((elapsed2 / 10000) * 1000).toFixed(1)}μs/promise)`,
      nestedAsync: `${100 * 100} nested calls in ${elapsed3}ms (${((elapsed3 / 10000) * 1000).toFixed(1)}μs/call)`,
    };
  },
});
