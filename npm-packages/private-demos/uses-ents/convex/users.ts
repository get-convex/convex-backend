import { v } from "convex/values";
import { mutation, query, getMutationTable, getQueryTable } from "./functions";

export const createUser = mutation({
  args: {
    name: v.string(),
    email: v.string(),
    bio: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const table = getMutationTable(ctx);
    const userId = await table("users").insert(args);
    return userId;
  },
});

export const getUserByEmail = query({
  args: { email: v.string() },
  handler: async (ctx, { email }) => {
    const table = getQueryTable(ctx);
    const user = await table("users").get("email", email);
    if (!user) return null;
    return {
      _id: user._id,
      name: user.name,
      email: user.email,
      bio: user.bio,
    };
  },
});

export const getUserWithPosts = query({
  args: { userId: v.id("users") },
  handler: async (ctx, { userId }) => {
    const table = getQueryTable(ctx);
    const user = await table("users").getX(userId);
    const posts = await user.edge("posts");

    return {
      _id: user._id,
      name: user.name,
      email: user.email,
      bio: user.bio,
      posts: await Promise.all(
        posts.map(async (post) => ({
          _id: post._id,
          title: post.title,
          slug: post.slug,
          published: post.published,
          createdAt: post.createdAt,
        })),
      ),
    };
  },
});

export const listUsers = query({
  args: {},
  handler: async (ctx) => {
    const table = getQueryTable(ctx);
    return await table("users").map(async (user) => ({
      _id: user._id,
      name: user.name,
      email: user.email,
      bio: user.bio,
    }));
  },
});
