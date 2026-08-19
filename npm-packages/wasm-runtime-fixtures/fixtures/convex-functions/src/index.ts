// Felix entry point for the Convex fixture.
//
// The Convex functions themselves live in `src/convex` and are written exactly
// as they would be in a real app. The backend they run against is the real
// `convex-test` package: it implements the database, indexes, validators,
// scheduler, and `runQuery`/`runMutation` in JS, so Felix does not have to
// reimplement any of it. What Felix supplies is the JS environment
// (`./runtime-shims`) plus the host `fetch` that actions use.
//
// `convex-test` keeps its data in the QuickJS heap, so state is shared by every
// invocation on one instance and disappears when the instance is torn down.
import "./runtime-shims";

import { convexTest } from "convex-test";

import { api, internal } from "./convex/_generated/api";
import * as generatedApi from "./convex/_generated/api";
import * as generatedServer from "./convex/_generated/server";
import * as messages from "./convex/messages";
import schema from "./convex/schema";
import * as schemaModule from "./convex/schema";

// The equivalent of the `import.meta.glob("./convex/**/*.ts")` a Vitest setup
// would pass; the bundle is static, so the modules are listed by hand.
const modules = {
  "./convex/schema.ts": () => Promise.resolve(schemaModule),
  "./convex/messages.ts": () => Promise.resolve(messages),
  "./convex/_generated/api.ts": () => Promise.resolve(generatedApi),
  "./convex/_generated/server.ts": () => Promise.resolve(generatedServer),
};

type ConvexTestClient = ReturnType<typeof convexTest>;

let client: ConvexTestClient | null = null;

function backend(): ConvexTestClient {
  client ??= convexTest(schema, modules);
  return client;
}

// Queries -------------------------------------------------------------------

export function listMessages(args: { channel: string; limit?: number }) {
  return backend().query(api.messages.list, args);
}

export function getMessage(args: { id: string }) {
  return backend().query(api.messages.get, args);
}

export function channelStats(args: { channel: string }) {
  return backend().query(api.messages.stats, args);
}

export function searchMessages(args: { channel: string; needle: string }) {
  return backend().query(api.messages.search, args);
}

// Mutations -----------------------------------------------------------------

export function sendMessage(args: {
  channel: string;
  author: string;
  body: string;
  tags?: string[];
}) {
  return backend().mutation(api.messages.send, args);
}

export function editMessage(args: { id: string; body: string }) {
  return backend().mutation(api.messages.edit, args);
}

export function deleteMessage(args: { id: string }) {
  return backend().mutation(api.messages.remove, args);
}

export function purgeChannel(args: { channel: string }) {
  return backend().mutation(api.messages.purgeChannel, args);
}

// Actions -------------------------------------------------------------------

export function summarizeChannel(args: {
  channel: string;
  summarizerUrl: string;
}) {
  return backend().action(api.messages.summarize, args);
}

export function importMessages(args: { channel: string; sourceUrl: string }) {
  return backend().action(api.messages.importFromUrl, args);
}

export function fanOutFetches(args: { urls: string[] }) {
  return backend().action(api.messages.fanOutFetches, args);
}

// A whole scenario in one request, which is the shape a Felix request actually
// has: fresh instance, seed, exercise, tear down.
export async function conversationScenario(args: { channel: string }) {
  const t = convexTest(schema, modules);

  await t.mutation(api.messages.send, {
    channel: args.channel,
    author: "emma",
    body: "hello",
  });
  const second = await t.mutation(api.messages.send, {
    channel: args.channel,
    author: "sujay",
    body: "hi there",
    tags: ["greeting"],
  });
  await t.mutation(api.messages.send, {
    channel: "other",
    author: "emma",
    body: "off topic",
  });
  await t.mutation(api.messages.edit, { id: second, body: "hi there!" });

  const lines: string[] = await t.query(internal.messages.linesForSummary, {
    channel: args.channel,
  });

  return {
    stats: await t.query(api.messages.stats, { channel: args.channel }),
    listed: await t.query(api.messages.list, { channel: args.channel }),
    matches: await t.query(api.messages.search, {
      channel: args.channel,
      needle: "HELLO",
    }),
    lines,
  };
}

// Direct access to the fake backend, the way a test would seed data.
export async function seedAndCount(args: { channel: string; count: number }) {
  const t = convexTest(schema, modules);

  await t.run(async (ctx) => {
    for (let index = 0; index < args.count; index += 1) {
      await ctx.db.insert("messages", {
        channel: args.channel,
        author: `author${index}`,
        body: `message ${index}`,
        tags: [],
        edited: false,
      });
    }
  });

  return t.query(api.messages.stats, { channel: args.channel });
}
