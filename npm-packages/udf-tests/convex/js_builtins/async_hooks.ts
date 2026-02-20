/**
 * Tests for AsyncLocalStorage and AsyncResource implementations.
 */

import { action } from "../_generated/server";
import { assert } from "chai";
import { wrapInTests } from "./testHelpers";

// AsyncLocalStorage and AsyncResource are set up as globals by setupAsyncHooks in the runtime
// We declare minimal type interfaces here for testing purposes

interface AsyncLocalStorageConstructor {
  new <T = unknown>(): AsyncLocalStorageInstance<T>;
  bind<F extends (...args: any[]) => any>(fn: F, ...args: any[]): F;
  snapshot(): <R>(fn: (...args: any[]) => R, ...args: any[]) => R;
}

interface AsyncLocalStorageInstance<T> {
  run<R>(store: T, callback: (...args: any[]) => R, ...args: any[]): R;
  exit<R>(callback: (...args: any[]) => R, ...args: any[]): R;
  getStore(): T | undefined;
  enterWith(store: T): void;
  disable(): void;
}

interface AsyncResourceConstructor {
  new (
    type: string,
    options?: { triggerAsyncId?: number } | number,
  ): AsyncResourceInstance;
  bind<F extends (...args: any[]) => any>(
    fn: F,
    type?: string,
    thisArg?: any,
  ): F;
}

interface AsyncResourceInstance {
  readonly type: string;
  runInAsyncScope<R>(
    fn: (...args: any[]) => R,
    thisArg?: any,
    ...args: any[]
  ): R;
  bind<F extends (...args: any[]) => any>(fn: F, thisArg?: any): F;
}

declare const AsyncLocalStorage: AsyncLocalStorageConstructor;
declare const AsyncResource: AsyncResourceConstructor;

/**
 * Test basic run() and getStore() functionality
 */
const basicRunAndGetStore = async () => {
  const als = new AsyncLocalStorage<string>();

  // Outside of run, getStore returns undefined
  assert.strictEqual(als.getStore(), undefined);

  // Inside run, getStore returns the store value
  als.run("test-value", () => {
    assert.strictEqual(als.getStore(), "test-value");
  });

  // After run, getStore returns undefined again
  assert.strictEqual(als.getStore(), undefined);
};

/**
 * Test that context is preserved across await
 */
const contextPreservedAcrossAwait = async () => {
  const als = new AsyncLocalStorage<{ userId: string }>();

  await als.run({ userId: "123" }, async () => {
    assert.deepEqual(als.getStore(), { userId: "123" });

    await Promise.resolve();
    assert.deepEqual(als.getStore(), { userId: "123" });

    await new Promise((resolve) => setTimeout(resolve, 10));
    assert.deepEqual(als.getStore(), { userId: "123" });
  });
};

/**
 * Test nested run() calls
 */
const nestedRunCalls = async () => {
  const als = new AsyncLocalStorage<number>();

  als.run(1, () => {
    assert.strictEqual(als.getStore(), 1);

    als.run(2, () => {
      assert.strictEqual(als.getStore(), 2);

      als.run(3, () => {
        assert.strictEqual(als.getStore(), 3);
      });

      assert.strictEqual(als.getStore(), 2);
    });

    assert.strictEqual(als.getStore(), 1);
  });

  assert.strictEqual(als.getStore(), undefined);
};

/**
 * Test multiple AsyncLocalStorage instances
 */
const multipleInstances = async () => {
  const als1 = new AsyncLocalStorage<string>();
  const als2 = new AsyncLocalStorage<number>();

  als1.run("hello", () => {
    als2.run(42, () => {
      assert.strictEqual(als1.getStore(), "hello");
      assert.strictEqual(als2.getStore(), 42);
    });
    assert.strictEqual(als1.getStore(), "hello");
    assert.strictEqual(als2.getStore(), undefined);
  });
};

/**
 * Test exit() method
 */
const exitMethod = async () => {
  const als = new AsyncLocalStorage<string>();

  als.run("outer", () => {
    assert.strictEqual(als.getStore(), "outer");

    const result = als.exit(() => {
      assert.strictEqual(als.getStore(), undefined);
      return "exit-result";
    });

    assert.strictEqual(result, "exit-result");
    assert.strictEqual(als.getStore(), "outer");
  });
};

/**
 * Test enterWith() method
 */
const enterWithMethod = async () => {
  const als = new AsyncLocalStorage<string>();

  await als.run("initial", async () => {
    assert.strictEqual(als.getStore(), "initial");

    als.enterWith("entered");
    assert.strictEqual(als.getStore(), "entered");

    await Promise.resolve();
    assert.strictEqual(als.getStore(), "entered");
  });
};

/**
 * Test disable() method
 */
const disableMethod = async () => {
  const als = new AsyncLocalStorage<string>();

  als.run("value", () => {
    assert.strictEqual(als.getStore(), "value");

    als.disable();
    assert.strictEqual(als.getStore(), undefined);

    // Re-enable by calling run
    als.run("new-value", () => {
      assert.strictEqual(als.getStore(), "new-value");
    });
  });
};

/**
 * Test static snapshot() method
 */
const snapshotMethod = async () => {
  const als = new AsyncLocalStorage<string>();

  let capturedSnapshot: ReturnType<typeof AsyncLocalStorage.snapshot>;

  als.run("snapshot-value", () => {
    capturedSnapshot = AsyncLocalStorage.snapshot();
  });

  // Outside the run context
  assert.strictEqual(als.getStore(), undefined);

  // Use the snapshot to restore context
  const result = capturedSnapshot!((x: number) => {
    assert.strictEqual(als.getStore(), "snapshot-value");
    return x * 2;
  }, 21);

  assert.strictEqual(result, 42);
  assert.strictEqual(als.getStore(), undefined);
};

/**
 * Test static bind() method
 */
const bindMethod = async () => {
  const als = new AsyncLocalStorage<string>();

  let boundFn: (suffix: string) => string;

  als.run("bound-value", () => {
    boundFn = AsyncLocalStorage.bind((suffix: string) => {
      return `${als.getStore()}-${suffix}`;
    });
  });

  // Outside the run context
  assert.strictEqual(als.getStore(), undefined);

  // The bound function should restore the captured context
  const result = boundFn!("suffix");
  assert.strictEqual(result, "bound-value-suffix");
};

/**
 * Test run() with arguments
 */
const runWithArguments = async () => {
  const als = new AsyncLocalStorage<string>();

  const result = als.run(
    "context",
    (a: number, b: number) => {
      assert.strictEqual(als.getStore(), "context");
      return a + b;
    },
    10,
    20,
  );

  assert.strictEqual(result, 30);
};

/**
 * Test context propagation through Promise.then chains
 */
const promiseThenChain = async () => {
  const als = new AsyncLocalStorage<number>();

  await als.run(100, async () => {
    const result = await Promise.resolve(1)
      .then((x) => {
        assert.strictEqual(als.getStore(), 100);
        return x + 1;
      })
      .then((x) => {
        assert.strictEqual(als.getStore(), 100);
        return x + 1;
      })
      .then((x) => {
        assert.strictEqual(als.getStore(), 100);
        return x + 1;
      });

    assert.strictEqual(result, 4);
  });
};

/**
 * Test context propagation through Promise.catch
 */
const promiseCatch = async () => {
  const als = new AsyncLocalStorage<string>();

  await als.run("error-context", async () => {
    const result = await Promise.reject(new Error("test")).catch((err) => {
      assert.strictEqual(als.getStore(), "error-context");
      return `caught: ${err.message}`;
    });

    assert.strictEqual(result, "caught: test");
  });
};

/**
 * Test context propagation through Promise.finally
 */
const promiseFinally = async () => {
  const als = new AsyncLocalStorage<string>();
  let finallyCalled = false;

  await als.run("finally-context", async () => {
    await Promise.resolve().finally(() => {
      assert.strictEqual(als.getStore(), "finally-context");
      finallyCalled = true;
    });
  });

  assert.isTrue(finallyCalled);
};

/**
 * Test that AsyncLocalStorage does not patch Promise.prototype.
 */
const promisePrototypeRemainsNative = async () => {
  const beforeThen = Function.prototype.toString.call(Promise.prototype.then);
  const beforeCatch = Function.prototype.toString.call(Promise.prototype.catch);
  const beforeFinally = Function.prototype.toString.call(
    Promise.prototype.finally,
  );

  const als = new AsyncLocalStorage<string>();
  await als.run("no-patch", async () => {
    await Promise.resolve();
    assert.strictEqual(als.getStore(), "no-patch");
  });

  assert.strictEqual(
    Function.prototype.toString.call(Promise.prototype.then),
    beforeThen,
  );
  assert.strictEqual(
    Function.prototype.toString.call(Promise.prototype.catch),
    beforeCatch,
  );
  assert.strictEqual(
    Function.prototype.toString.call(Promise.prototype.finally),
    beforeFinally,
  );
};

/**
 * Test AsyncResource basic functionality
 */
const asyncResourceBasic = async () => {
  const als = new AsyncLocalStorage<string>();

  let resource: InstanceType<typeof AsyncResource>;

  als.run("resource-context", () => {
    resource = new AsyncResource("TestResource");
  });

  // Outside the run context
  assert.strictEqual(als.getStore(), undefined);

  // Use runInAsyncScope to restore context
  const result = resource!.runInAsyncScope(() => {
    assert.strictEqual(als.getStore(), "resource-context");
    return "success";
  });

  assert.strictEqual(result, "success");
  assert.strictEqual(als.getStore(), undefined);
};

/**
 * Test AsyncResource.bind()
 */
const asyncResourceBind = async () => {
  const als = new AsyncLocalStorage<string>();

  let boundFn: () => string;

  als.run("bind-context", () => {
    const resource = new AsyncResource("TestResource");
    boundFn = resource.bind(() => {
      return als.getStore() || "no-store";
    });
  });

  // Outside the run context
  assert.strictEqual(als.getStore(), undefined);

  // The bound function restores context
  const result = boundFn!();
  assert.strictEqual(result, "bind-context");
};

/**
 * Test AsyncResource static bind
 */
const asyncResourceStaticBind = async () => {
  const als = new AsyncLocalStorage<string>();

  let boundFn: () => string;

  als.run("static-bind", () => {
    boundFn = AsyncResource.bind(() => {
      return als.getStore() || "no-store";
    });
  });

  const result = boundFn!();
  assert.strictEqual(result, "static-bind");
};

/**
 * Test that context is isolated between concurrent async operations
 */
const concurrentIsolation = async () => {
  const als = new AsyncLocalStorage<number>();

  const results: number[] = [];

  const task = (id: number, delay: number): Promise<void> => {
    return als.run(id, async () => {
      await new Promise((resolve) => setTimeout(resolve, delay));
      results.push(als.getStore()!);
    });
  };

  await Promise.all([task(1, 30), task(2, 20), task(3, 10)]);

  // Each task should have its own context despite running concurrently
  assert.sameMembers(results, [1, 2, 3]);
};

/**
 * Test that run() handles exceptions properly
 */
const runWithException = async () => {
  const als = new AsyncLocalStorage<string>();

  let contextAfterException: string | undefined;

  try {
    als.run("before-throw", () => {
      throw new Error("test error");
    });
  } catch {
    // Context should be restored after exception
    contextAfterException = als.getStore();
  }

  assert.strictEqual(contextAfterException, undefined);
};

/**
 * Test async run with exception
 */
const asyncRunWithException = async () => {
  const als = new AsyncLocalStorage<string>();

  try {
    await als.run("async-throw", async () => {
      await Promise.resolve();
      throw new Error("async error");
    });
  } catch {
    // Expected
  }

  assert.strictEqual(als.getStore(), undefined);
};

/**
 * Test context preservation across multiple sequential Rust async ops.
 * This explicitly verifies continuation-preserved async context propagation
 * when promises are created and resolved by the Rust runtime (via setTimeout,
 * which uses V8's PromiseResolver).
 */
const contextAcrossMultipleAsyncOps = async () => {
  const als = new AsyncLocalStorage<string>();

  await als.run("rust-async-ops", async () => {
    // First Rust async op (setTimeout creates a promise resolved by Rust)
    await new Promise((resolve) => setTimeout(resolve, 5));
    assert.strictEqual(als.getStore(), "rust-async-ops");

    // Second Rust async op
    await new Promise((resolve) => setTimeout(resolve, 5));
    assert.strictEqual(als.getStore(), "rust-async-ops");

    // Mix of pure JS promise and Rust async op
    await Promise.resolve();
    assert.strictEqual(als.getStore(), "rust-async-ops");

    // Third Rust async op
    await new Promise((resolve) => setTimeout(resolve, 5));
    assert.strictEqual(als.getStore(), "rust-async-ops");

    // Nested async function call
    const nestedAsync = async () => {
      await new Promise((resolve) => setTimeout(resolve, 5));
      return als.getStore();
    };
    const result = await nestedAsync();
    assert.strictEqual(result, "rust-async-ops");
  });
};

/**
 * Test that interleaved async ops maintain separate contexts.
 * This verifies context isolation when multiple async operations
 * are in flight simultaneously.
 */
const interleavedAsyncOps = async () => {
  const als = new AsyncLocalStorage<number>();
  const results: Array<{
    id: number;
    step: number;
    store: number | undefined;
  }> = [];

  const task = async (id: number) => {
    await als.run(id, async () => {
      results.push({ id, step: 1, store: als.getStore() });
      await new Promise((resolve) => setTimeout(resolve, 10 * id));
      results.push({ id, step: 2, store: als.getStore() });
      await new Promise((resolve) => setTimeout(resolve, 5));
      results.push({ id, step: 3, store: als.getStore() });
    });
  };

  // Start tasks that will interleave due to different delays
  await Promise.all([task(1), task(2), task(3)]);

  // Verify each task's results have the correct context
  for (const result of results) {
    assert.strictEqual(
      result.store,
      result.id,
      `Task ${result.id} step ${result.step} had wrong context`,
    );
  }
};

export default action(async () => {
  return await wrapInTests({
    basicRunAndGetStore,
    contextPreservedAcrossAwait,
    nestedRunCalls,
    multipleInstances,
    exitMethod,
    enterWithMethod,
    disableMethod,
    snapshotMethod,
    bindMethod,
    runWithArguments,
    promiseThenChain,
    promiseCatch,
    promiseFinally,
    promisePrototypeRemainsNative,
    asyncResourceBasic,
    asyncResourceBind,
    asyncResourceStaticBind,
    concurrentIsolation,
    runWithException,
    asyncRunWithException,
    contextAcrossMultipleAsyncOps,
    interleavedAsyncOps,
  });
});
