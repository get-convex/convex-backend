import { query, internalMutation } from "./_generated/server";
import { v } from "convex/values";

export const list = query({
  args: {},
  returns: v.array(
    v.object({
      _id: v.id("posts"),
      _creationTime: v.number(),
      title: v.string(),
      content: v.string(),
      author: v.string(),
    }),
  ),
  handler: async (ctx) => {
    const posts = await ctx.db.query("posts").order("desc").collect();
    return posts;
  },
});

export const add = internalMutation({
  args: {
    title: v.string(),
    content: v.string(),
    author: v.string(),
  },
  returns: v.id("posts"),
  handler: async (ctx, args) => {
    const postId = await ctx.db.insert("posts", {
      title: args.title,
      content: args.content,
      author: args.author,
    });
    return postId;
  },
});
