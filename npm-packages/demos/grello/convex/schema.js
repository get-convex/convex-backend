import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

function listAsLiterals(l) {
  return v.union(...l.map((i) => v.literal(i)));
}

export default defineSchema({
  apiKeys: defineTable({
    key: v.string(),
    isEnabled: v.boolean(),
  }),

  // jwt
  workspaces: defineTable({
    slug: v.string(),
  }).index("bySlug", ["slug"]),

  buckets: defineTable({
    workspace: v.id("workspaces"),
    slug: v.string(),
    search: v.string(),
    lastUpdate: v.number(),
    cursor: v.string(),
  }).index("byWorkspace", ["workspace"]),

  kart: defineTable({
    workspace: v.id("workspaces"),
    filename: v.string(),
    injested: v.string(),
  }).index("byWorkspace", ["workspace"]),

  cards: defineTable({
    workspace: v.id("workspaces"),
    parent: v.id("cards"),
    source: v.string(),
    kind: listAsLiterals(["task", "event"]),
    slug: v.string(),
  }).index("byWorkspace", ["workspace"]),

  cardTag: defineTable({
    workspace: v.id("workspaces"),
    card: v.id("cards"),
    tag: v.string(),
  })
    .index("byWorkspace", ["workspace"])
    .index("byCard", ["workspace", "card"])
    .index("byTag", ["workspace", "tag"]),
});
