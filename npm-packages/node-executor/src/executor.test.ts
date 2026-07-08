import { describe, expect, test } from "vitest";
import { runWithEnvironmentVariables } from "./executor";

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

async function tick() {
  await new Promise<void>((resolve) => setImmediate(resolve));
}

describe("runWithEnvironmentVariables", () => {
  test("allows overlapping invocations with the same env hash", async () => {
    const firstStarted = deferred();
    const releaseFirst = deferred();
    let secondStarted = false;

    const envs = [{ name: "CONVEX_TEST_SHARED", value: "shared" }];
    const first = runWithEnvironmentVariables([...envs], async () => {
      expect(process.env.CONVEX_TEST_SHARED).toBe("shared");
      firstStarted.resolve();
      await releaseFirst.promise;
      expect(process.env.CONVEX_TEST_SHARED).toBe("shared");
    });

    await firstStarted.promise;

    const second = runWithEnvironmentVariables([...envs], async () => {
      secondStarted = true;
      expect(process.env.CONVEX_TEST_SHARED).toBe("shared");
    });

    await tick();

    expect(secondStarted).toBe(true);
    releaseFirst.resolve();
    await Promise.all([first, second]);
  });

  test("waits for the active batch before using a different env hash", async () => {
    const firstStarted = deferred();
    const releaseFirst = deferred();
    let secondStarted = false;

    const first = runWithEnvironmentVariables(
      [{ name: "CONVEX_TEST_STALE", value: "stale" }],
      async () => {
        expect(process.env.CONVEX_TEST_STALE).toBe("stale");
        firstStarted.resolve();
        await releaseFirst.promise;
        expect(process.env.CONVEX_TEST_STALE).toBe("stale");
      },
    );

    await firstStarted.promise;

    const second = runWithEnvironmentVariables(
      [{ name: "CONVEX_TEST_FRESH", value: "fresh" }],
      async () => {
        secondStarted = true;
        expect(process.env.CONVEX_TEST_STALE).toBeUndefined();
        expect(process.env.CONVEX_TEST_FRESH).toBe("fresh");
      },
    );

    await tick();

    expect(secondStarted).toBe(false);
    releaseFirst.resolve();
    await Promise.all([first, second]);
    expect(secondStarted).toBe(true);
  });
});
