import { v } from "convex/values";
import { defineEnt, defineEntSchema, getEntDefinitions } from "convex-ents";

const schema = defineEntSchema({
  users: defineEnt({
    name: v.string(),
    bio: v.optional(v.string()),
  })
    .field("email", v.string(), { unique: true })
    .edges("posts", { ref: true })
    .edges("comments", { ref: true }),

  posts: defineEnt({
    title: v.string(),
    content: v.string(),
    published: v.boolean(),
    createdAt: v.number(),
  })
    .field("slug", v.string(), { unique: true })
    .edge("author", { to: "users" })
    .edges("comments", { ref: true })
    .edges("tags"),

  comments: defineEnt({
    text: v.string(),
    createdAt: v.number(),
  })
    .edge("post", { to: "posts" })
    .edge("author", { to: "users" }),

  tags: defineEnt({
    name: v.string(),
  })
    .field("slug", v.string(), { unique: true })
    .edges("posts"),
});

export default schema;

export const entDefinitions = getEntDefinitions(schema);
