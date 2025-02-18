// This file is doing all kinds of wonky things so just turn off eslint
/* eslint-disable */
import { api } from "./_generated/api";
import { Id } from "./_generated/dataModel";
import { action, mutation, query } from "./_generated/server";

export const simpleLoop = query(() => {
  while (1) {}
});

export const slow = query(() => {
  for (let i = 0; i < 120000; i++) {
    crypto.randomUUID();
  }
});

export const populate = mutation(async ({ db }) => {
  for (let i = 0; i < 16; i++) {
    await db.insert("test", { counter: i });
  }
});

export const dbLoop = query(async ({ db }) => {
  let _numRows = 0;
  while (1) {
    for await (const _row of db.query("test")) {
      _numRows += 1;
    }
  }
});

export const consoleLoop = query(() => {
  for (let i = 0; i < 258; i++) {
    console.log("hello there");
  }
});

export const consoleLoopTimes = query((_ctx, { times }: { times: number }) => {
  for (let i = 0; i < times; i++) {
    console.log("are we there yet");
  }
});

export const consoleLoopFromSubfunction = query(async (ctx) => {
  await ctx.runQuery(api.adversarial.consoleLoopTimes, { times: 200 });
  console.log("we get there when we get there");
  await ctx.runQuery(api.adversarial.consoleLoopTimes, { times: 100 });
  console.log("we get there when we get there");
});

export const consoleLongLine = query(() => {
  // We truncate the output of console.log to 32768 bytes, test this
  // https://en.uncyclopedia.co/wiki/AAAAAAAAA!
  const longLine = "A".repeat(4511) + "!";
  const args = Array(9).fill(longLine);
  console.log(...args);
  console.log(args.join(" "));
});

export const consoleLongLineCharBoundary = query(() => {
  // We truncate the output of console.log to 32768 bytes
  // Use a 4 byte emoji with different amounts of padding to ensure
  // that we respect character boundaries when truncating.
  const longLine = "🙃".repeat(8192);
  console.log("x" + longLine);
  console.log("xx" + longLine);
  console.log("xxx" + longLine);
  console.log("xxxx" + longLine);
});

export const queryLeak = query(async ({ db }) => {
  while (1) {
    for await (const _row of db.query("test")) {
      break;
    }
  }
});

export const queryATon = query(async ({ db }) => {
  for (let i = 0; i < 15000; i++) {
    for await (const _row of db.query("test")) {
      break;
    }
  }
});

export const queryTooManyTimes = query(async ({ db }) => {
  for (let i = 0; i < 5000; i++) {
    await db
      .query("test")
      .withIndex("by_hello", (q) => q.eq("hello", i))
      .collect();
  }
});

export const queryManyTimes = query(async ({ db }) => {
  for (let i = 0; i < 4000; i++) {
    await db
      .query("test")
      .withIndex("by_hello", (q) => q.eq("hello", i))
      .collect();
  }
});

// Based on https://developer.mozilla.org/en-US/docs/Web/API
export const tryUnsupportedAPIs = query(() => {
  // These should all exist, but be stubbed out when used (see tests lower down)
  let x: any = setTimeout;
  x = setInterval;

  if (typeof Worker !== "undefined") {
    throw new Error("Worker defined");
  }
  if (typeof RTCPeerConnection !== "undefined") {
    throw new Error("RTCPeerConnection defined");
  }
  if (typeof WebSocket !== "undefined") {
    throw new Error("WebSocket defined");
  }
  if (typeof XMLHttpRequest !== "undefined") {
    throw new Error("XMLHttpRequest defined");
  }
  if (typeof document !== "undefined") {
    throw new Error("document defined");
  }
  if (typeof window !== "undefined") {
    throw new Error("window defined");
  }
  if (typeof indexedDB !== "undefined") {
    throw new Error("indexedDB defined");
  }
  if (typeof caches !== "undefined") {
    throw new Error("caches defined");
  }
  if (typeof localStorage !== "undefined") {
    throw new Error("localStorage defined");
  }
  if (typeof sessionStorage !== "undefined") {
    throw new Error("sessionStorage defined");
  }
  if (typeof queueMicrotask !== "undefined") {
    throw new Error("queueMicrotask defined");
  }
  if (typeof requestIdleCallback !== "undefined") {
    throw new Error("requestIdleCallback defined");
  }
  if (typeof requestAnimationFrame !== "undefined") {
    throw new Error("requestAnimationFrame defined");
  }
  if (typeof setImmediate !== "undefined") {
    throw new Error("setImmediate defined");
  }
  if (typeof performance !== "undefined") {
    throw new Error("performance defined");
  }
});

export const setTimeoutThrows = query(() => {
  try {
    setTimeout(() => {
      console.log("timeout");
    }, 1000);
  } catch (e) {
    // Error should not be catchable
    return "Caught an error";
  }
});

export const setIntervalThrows = query(() => {
  try {
    setInterval(() => {
      console.log("interval");
    }, 1000);
  } catch (e) {
    // Error should not be catchable
    return "Caught an error";
  }
});

export const populateBigRead = mutation(async ({ db }) => {
  const ids = [];
  for (let i = 0; i < 4097; i++) {
    const id = await db.insert("test", { counter: i });
    ids.push(id);
  }
  return ids;
});

export const bigRead = query(async ({ db }, { ids }: { ids: Id<any>[] }) => {
  for (const id of ids) {
    await db.get(id);
  }
});

export const readUntilError = query(async ({ db }) => {
  const documents = [];
  try {
    for await (const doc of db.query("test")) {
      documents.push(doc);
    }
  } catch (e: any) {}
  return documents;
});

export const bigWrite = mutation(
  async ({ db }, { count }: { count: number }) => {
    const bytes = new ArrayBuffer(262144);
    const view = new BigInt64Array(bytes);
    view.fill(1017n);

    let ids = [];
    for (let i = 0; i < count; i++) {
      ids.push(await db.insert("test", { data: bytes }));
    }
    return ids;
  },
);

export const bigDocument = mutation(async (ctx) => {
  await ctx.db.insert("test", { data: "something small" });
  let big = "";
  for (let i = 0; i < 1000000; i++) {
    big += "A";
  }
  const bigId = await ctx.db.insert("test", { data: big });
  await ctx.db.insert("test", { data: "something else small" });
  return bigId;
});

export const nestedDocument = mutation(async (ctx) => {
  await ctx.db.insert("test", { data: [[[]]] });
  let big = "";
  for (let i = 0; i < 1000000; i++) {
    big += "A";
  }
  const nestedId = await ctx.db.insert("test", {
    data: [[[[[[[[[[[[[[]]]]]]]]]]]]]],
  });
  await ctx.db.insert("test", { data: [[[[[]]]]] });
  return nestedId;
});

export const oom = query(async ({ db }) => {
  let s = "";
  for (let i = 0; i < 8340000; i++) {
    s += ("" + i)[0];
  }
  return s.length;
});

export const tooManyWrites = mutation(async ({ db }) => {
  for (let i = 0; i < 8193; i++) {
    await db.insert("test", { counter: i });
  }
});

export const manyWrites = mutation(async ({ db }) => {
  for (let i = 0; i < 8093; i++) {
    await db.insert("test", { counter: i });
  }
});

export const returnTooLarge = query(({ db }) => {
  return Array(9000).fill(1);
});

export const iterateTwice = query(async ({ db }) => {
  const query = db.query("test").fullTableScan();
  for await (const _ of query) {
  }
  for await (const _ of query) {
  }
});

export const iterateConsumed = query(async ({ db }) => {
  const query = db.query("test").fullTableScan();
  query.take(1);
  for await (const _ of query) {
  }
});

// export const tryEval = mutation(async () => {
//   return eval("1 + 1");
// });

export const tryNewFunction = mutation(async () => {
  return new Function("return 3")();
});

declare const Convex: {
  syscall: (op: string, jsonArgs: string) => string;
  asyncSyscall: (op: string, jsonArgs: string) => Promise<string>;
  jsSyscall: (op: string, args: Record<string, any>) => any;
};

export const deleteConvexGlobal = query(async ({ db }) => {
  // @ts-expect-error -- error TS2790: The operand of a 'delete' operator must be optional.
  delete Convex.syscall;
  return await db.query("test").collect();
});

export const throwSystemError = query(async () => {
  Convex.syscall("throwSystemError", "{}");
});

export const throwSystemErrorAfterAwait = query(async () => {
  await Promise.resolve(null);
  Convex.syscall("throwSystemError", "{}");
});

export const throwUncatchableDeveloperError = query(async () => {
  try {
    Convex.jsSyscall("idonotexistandicannotlie", {});
  } catch (e) {
    console.log("caught you", e);
  }
});

export const slowSyscall = query(async () => {
  return JSON.parse(await Convex.asyncSyscall("slowSyscall", "{}"));
});

export const reallySlowSyscall = query(async () => {
  return JSON.parse(await Convex.asyncSyscall("reallySlowSyscall", "{}"));
});

export const atomicsWait = query(async () => {
  const buffer = new SharedArrayBuffer(4);
  const array = new Int32Array(buffer);
  Atomics.wait(array, 0, 1);
});

export const bigMemoryUsage = query(async () => {
  // javascript strings are utf16 so 2 bytes per character
  // max is 64MB but there's a buffer that brings it up to 100MB.
  // note we can't use `new ArrayBuffer` because ArrayBuffers don't count
  // as used memory in v8.
  let ten = "10bts";
  let s = "";
  for (let i = 0; i < 2 * 1000 * 1000; i++) {
    s += ten;
  }
  return s.length;
});

export const useNotImplementedBuiltin = query(async () => {
  try {
    const url = new URL("https://baz.qat:8000/qux/quux?foo=bar&baz=12#qat");
    // This should throw an uncatchable "Not implemented" error
    url.password;
  } catch (e) {
    return "Caught an error!";
  }
});

export const insertWithCreationTime = mutation(async ({ db }) => {
  await db.insert("table", {
    _creationTime: 123,
  });
});

export const insertWithId = mutation(async ({ db }) => {
  await db.insert("table", {
    _id: 123,
  });
});

export const queryResultSized = query(async (_, { size }: { size: number }) => {
  let x = new ArrayBuffer(size);
  return x;
});

export const mutationResultSized = mutation(
  async (_, { size }: { size: number }) => {
    let x = new ArrayBuffer(size);
    return x;
  },
);

export const actionResultSized = action(
  async (_, { size }: { size: number }) => {
    let x = new ArrayBuffer(size);
    return x;
  },
);

export const simpleQuery = query(async ({ db }) => {
  return await db.query("test").collect();
});

export const invokeFunctionDirectly = query(async (ctx) => {
  await simpleQuery(ctx, {});
});
