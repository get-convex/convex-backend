"use node";

import {
  actionGeneric,
  internalActionGeneric,
  makeFunctionReference,
} from "convex/server";
import { api } from "./_generated/api";
import { ActionCtx } from "./_generated/server";

export const addNumbers = actionGeneric(
  async (_, { numbers }: { numbers: number[] }) => {
    let sum = 0;
    for (const arg of numbers) {
      sum += arg;
    }
    return sum;
  },
);

export const logHelloWorldAndReturn7 = actionGeneric(async () => {
  console.info("Hello");
  console.error("World!");
  return 7;
});

export const getUserIdentity = actionGeneric(async ({ auth }: ActionCtx) => {
  return auth.getUserIdentity();
});

export const runQuery = actionGeneric(
  async ({ runQuery }: ActionCtx, { name }: { name: string }) => {
    return runQuery(makeFunctionReference<"query">(name), {});
  },
);

export const scheduleJob = actionGeneric(
  async ({ scheduler }: ActionCtx, { name }: { name: string }) => {
    return scheduler.runAfter(
      0,
      makeFunctionReference<"mutation" | "action">(name),
      {},
    );
  },
);

export const logAndThrowError = actionGeneric(async () => {
  console.log("About to do something...");
  throw new Error("Oh, no!");
});

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export const sleepAnHour = actionGeneric(async () => {
  console.log("I am very sleepy. I am going to take a nap.");
  await sleep(3600 * 1000); // Sleep for an hour.
  return "slept a whole hour";
});

export const workHardForAnHour = actionGeneric(async () => {
  console.log("I am going to work really hard for 1 hour");
  const start = Date.now();
  while (Date.now() - start < 3600 * 1000) {
    // Spin loop.
  }
  return "I worked for a whole hour";
});

export const getTestEnvVar = actionGeneric(async () => {
  if (process.env.UNKNOWN_VAR !== undefined) {
    throw new Error("Unexpected environment variable defined for UNKNOWN_VAR");
  }
  return process.env.TEST_NAME;
});

export const deadlock = actionGeneric(async () => {
  // Deadlock the UDF
  return await new Promise(() =>
    setTimeout(() => {
      // intentional noop
    }, 500),
  );
});

export const internalHello = internalActionGeneric(async () => {
  console.log("analyze me pls");
});

// The action runs a mutation but doesn't await the outcome.
export const forgotAwait = actionGeneric(async ({ runMutation }: ActionCtx) => {
  // eslint-disable-next-line @typescript-eslint/no-floating-promises
  runMutation(api.basic.insertObject, { foo: "bar" });
});

export const actionCallsAction = actionGeneric(
  async ({ runAction }: ActionCtx) => {
    await runAction(api.node_actions.scheduleJob, { name: "getCounter.js" });
  },
);

// Returns a string with characters of a partial escape sequence
export const partialEscapeSequence = actionGeneric(async () => {
  return "\ud83c...";
});

export const echoMessage = actionGeneric(
  async (_, { message }: { message: string }) => {
    return message;
  },
);
