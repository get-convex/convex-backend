import { v } from "convex/values";
import { mutation, query, getMutationTable, getQueryTable } from "./functions";

export const createTag = mutation({
  args: {
    name: v.string(),
    slug: v.string(),
  },
  handler: async (ctx, args) => {
    const table = getMutationTable(ctx);
    const tagId = await table("tags").insert(args);
    return tagId;
  },
});

export const getTagBySlug = query({
  args: { slug: v.string() },
  handler: async (ctx, { slug }) => {
    const table = getQueryTable(ctx);
    const tag = await table("tags").get("slug", slug);
    if (!tag) return null;

    const posts = await tag.edge("posts");

    return {
      _id: tag._id,
      name: tag.name,
      slug: tag.slug,
      posts: await Promise.all(
        posts
          .filter((post) => post.published)
          .map(async (post) => {
            const author = await post.edge("author");
            return {
              _id: post._id,
              title: post.title,
              slug: post.slug,
              createdAt: post.createdAt,
              author: {
                _id: author._id,
                name: author.name,
              },
            };
          }),
      ),
    };
  },
});

export const listTags = query({
  args: {},
  handler: async (ctx) => {
    const table = getQueryTable(ctx);
    return await table("tags").map(async (tag) => ({
      _id: tag._id,
      name: tag.name,
      slug: tag.slug,
    }));
  },
});
