import { api } from "./_generated/api";
import { query, mutation } from "./_generated/server";
import { assert } from "chai";

export const logString = query(() => {
  console.log("myString");
});

export const logNumber = query(() => {
  console.log(42);
});

export const logUndefined = query(() => {
  console.log(undefined);
});

export const logNull = query(() => {
  console.log(null);
});

export const logFunction = query(() => {
  // eslint-disable-next-line @typescript-eslint/no-empty-function
  function myFunction() {}
  console.log(myFunction);
});

export const logInstance = query(() => {
  class MyClass {}
  console.log(new MyClass());
});

export const logObject = query(() => {
  console.log({
    property: "value",
    nested_object: {},
  });
});

export const logArray = query(() => {
  console.log(["string", 42]);
});

export const logDocument = mutation(async ({ db }) => {
  const id = await db.insert("table", { property: "value" });
  const document = await db.get(id);
  console.log(document);
});

export const logFromSubfunction = query(async (ctx) => {
  console.log("from parent");
  await ctx.runQuery(api.logging.logString, {});
});

export const consoleTrace = query(() => {
  console.trace("myString");
});

export const errorStack = query(() => {
  function innerFunc() {
    return new Error();
  }
  function outerFunc() {
    return innerFunc();
  }
  const stack = outerFunc().stack!.split("\n");
  assert.strictEqual(stack.length, 3);
  assert(stack[0].includes("    at innerFunc (../convex/logging.ts:"));
  assert(stack[1].includes("    at outerFunc (../convex/logging.ts:"));
  assert(stack[2].includes("    at <anonymous> (../convex/logging.ts:"));

  Error.stackTraceLimit = 2;
  const stackLimited = outerFunc().stack!.split("\n");
  assert.strictEqual(stackLimited.length, 2);
  assert(stackLimited[0].includes("    at innerFunc (../convex/logging.ts:"));
  assert(stackLimited[1].includes("    at outerFunc (../convex/logging.ts:"));

  function capture() {
    const trace = { type: "TRACE" } as any;
    Error.captureStackTrace(trace);
    return trace;
  }
  const trace = capture();
  assert.strictEqual(trace.type, "TRACE");
  const stackTrace = trace.stack.split("\n")!;
  assert.strictEqual(stackTrace.length, 2);
  assert(stackTrace[0].includes("    at capture (../convex/logging.ts:"));
  assert(stackTrace[1].includes("    at <anonymous> (../convex/logging.ts:"));
});

export const consoleTime = query(() => {
  console.time();
  console.timeLog(); // default: Xms
  console.timeEnd(); // default: Xms

  console.time("foo");
  console.time("foo"); // Timer "foo" already exists
  console.timeLog("foo", "bar", "baz"); // foo: Xms bar baz
  console.timeEnd("foo"); // foo: Xms
});
