// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// (for some tests)

import { action } from "../_generated/server";
import { v } from "convex/values";
import { assert } from "chai";
import { wrapInTests } from "./testHelpers";

export const sleep = action({
  args: { ms: v.number() },
  handler: (_ctx, args) => {
    return new Promise((resolve) => {
      setTimeout(() => {
        resolve("success");
      }, args.ms);
    });
  },
});

export const setTimeoutThrows = action(async () => {
  const [promise, resolve] = multiPromise(1);
  try {
    setTimeout(() => {
      resolve();
      throw new Error("THROWN WITHIN setTimeout");
    });
    await promise;
  } catch {
    throw new Error("This should not be catchable");
  }
});

export const danglingSetTimeout = action(async () => {
  setTimeout(() => {
    throw new Error("THROWN WITHIN setTimeout");
  }, 10000);
});

// returns promise that must be resolved n times.
const multiPromise = (n: number): [Promise<any>, () => void] => {
  let resolver: (v: any) => void;
  const promise = new Promise((resolve) => {
    resolver = resolve;
  });
  let resolveCount = 0;
  return [
    promise,
    () => {
      resolveCount += 1;
      if (resolveCount === n) {
        resolver(null);
      }
      if (resolveCount > n) {
        throw new Error("promise resolved too many times");
      }
    },
  ];
};

const setTimeoutArgs = async () => {
  const [promise, resolve] = multiPromise(4);
  let log = "";
  function logger(s: string) {
    log += s;
    resolve();
  }

  setTimeout(logger, 500, "2");
  setTimeout(logger, 500, "3");
  setTimeout(logger, 0, "1");
  setTimeout(logger, 1000, "4");
  await promise;
  assert.strictEqual(log, "1234");
};

const clearTimeoutTest = async () => {
  const [promise, resolve] = multiPromise(4);
  // t1 is cleared synchronously.
  const t1 = setTimeout(() => {
    throw new Error("cleared setTimeout ran");
  }, 500);
  // t2 is cleared after a shorter timeout.
  const t2 = setTimeout(() => {
    throw new Error("cleared setTimeout ran");
  }, 500);
  // t3 is cleared after it runs.
  const t3 = setTimeout(resolve, 100);
  // Make sure everything has time to run if it is going to.
  setTimeout(resolve, 1000);

  clearTimeout(t1);
  clearTimeout(t1); // silently ignore if already cleared.
  setTimeout(() => {
    clearTimeout(t2);
    resolve();
  }, 200);
  setTimeout(() => {
    clearTimeout(t3);
    resolve();
  }, 800);
  await promise;
};

const setIntervalTest = async () => {
  const [promise, resolve] = multiPromise(10);
  let counter = 0;
  setInterval(() => {
    counter += 1;
    if (counter <= 10) {
      resolve();
    }
  }, 100);
  await promise;
};

const clearIntervalTest = async () => {
  const [promise, resolve] = multiPromise(4);
  let counter = 0;
  const id = setInterval(() => {
    counter += 1;
    if (counter <= 3) {
      resolve();
    }
    if (counter === 3) {
      clearInterval(id);
    }
    if (counter > 3) {
      throw new Error("cleared setInterval ran");
    }
  }, 10) as unknown as number;
  // Wait for extra iterations to run if they are going to.
  setTimeout(resolve, 1000);
  await promise;
};

const timerBasicMicrotaskOrdering = async () => {
  let s = "";
  let count = 0;
  const [promise, resolve] = multiPromise(1);
  setTimeout(() => {
    Promise.resolve().then(
      () => {
        count++;
        s += "con";
        if (count === 2) {
          resolve();
        }
      },
      (e) => {
        throw new Error(e);
      },
    );
  });
  setTimeout(() => {
    count++;
    s += "vex";
    if (count === 2) {
      resolve();
    }
  });
  await promise;
  assert.strictEqual(s, "convex");
};

async function timerNestedMicrotaskOrdering() {
  let s = "";
  const [promise, resolve] = multiPromise(1);
  s += "0";
  setTimeout(() => {
    s += "4";
    setTimeout(() => (s += "A"));
    Promise.resolve()
      .then(
        () => {
          setTimeout(() => {
            s += "B";
            resolve();
          });
        },
        (e) => {
          throw new Error(e);
        },
      )
      .then(
        () => {
          s += "5";
        },
        (e) => {
          throw new Error(e);
        },
      );
  });
  setTimeout(() => (s += "6"));
  Promise.resolve().then(
    () => (s += "2"),
    (e) => {
      throw new Error(e);
    },
  );
  Promise.resolve().then(
    () =>
      setTimeout(() => {
        s += "7";
        Promise.resolve()
          .then(
            () => (s += "8"),
            (e) => {
              throw new Error(e);
            },
          )
          .then(
            () => {
              s += "9";
            },
            (e) => {
              throw new Error(e);
            },
          );
      }),
    (e) => {
      throw new Error(e);
    },
  );
  Promise.resolve().then(
    () => Promise.resolve().then(() => (s += "3")),
    (e) => {
      throw new Error(e);
    },
  );
  s += "1";
  await promise;
  assert.strictEqual(s, "0123456789AB");
}

export default action(async () => {
  return await wrapInTests({
    setTimeoutArgs,
    clearTimeoutTest,
    setIntervalTest,
    clearIntervalTest,
    timerBasicMicrotaskOrdering,
    timerNestedMicrotaskOrdering,
  });
});
