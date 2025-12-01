import { v } from "convex/values";
import { mutation, query, getMutationTable, getQueryTable } from "./functions";

export const createPost = mutation({
  args: {
    title: v.string(),
    slug: v.string(),
    content: v.string(),
    authorId: v.id("users"),
    published: v.boolean(),
    tagIds: v.optional(v.array(v.id("tags"))),
  },
  handler: async (ctx, { tagIds, ...args }) => {
    const table = getMutationTable(ctx);
    const postId = await table("posts").insert({
      ...args,
      createdAt: Date.now(),
      tags: tagIds,
    });
    return postId;
  },
});

export const getPostBySlug = query({
  args: { slug: v.string() },
  handler: async (ctx, { slug }) => {
    const table = getQueryTable(ctx);
    const post = await table("posts").get("slug", slug);
    if (!post) return null;

    const author = await post.edge("author");
    const tags = await post.edge("tags");
    const comments = await post.edge("comments");

    return {
      _id: post._id,
      title: post.title,
      slug: post.slug,
      content: post.content,
      published: post.published,
      createdAt: post.createdAt,
      author: {
        _id: author._id,
        name: author.name,
        email: author.email,
      },
      tags: tags.map((tag) => ({
        _id: tag._id,
        name: tag.name,
        slug: tag.slug,
      })),
      comments: await Promise.all(
        comments.map(async (comment) => {
          const commentAuthor = await comment.edge("author");
          return {
            _id: comment._id,
            text: comment.text,
            createdAt: comment.createdAt,
            author: {
              _id: commentAuthor._id,
              name: commentAuthor.name,
            },
          };
        }),
      ),
    };
  },
});

export const listPublishedPosts = query({
  args: {},
  handler: async (ctx) => {
    const table = getQueryTable(ctx);
    const posts = await table("posts")
      .filter((q) => q.eq(q.field("published"), true))
      .map(async (post) => {
        const author = await post.edge("author");
        const tags = await post.edge("tags");
        return {
          _id: post._id,
          title: post.title,
          slug: post.slug,
          content: post.content.substring(0, 200) + "...",
          createdAt: post.createdAt,
          author: {
            _id: author._id,
            name: author.name,
          },
          tags: tags.map((tag) => ({
            _id: tag._id,
            name: tag.name,
          })),
        };
      });
    return posts;
  },
});

export const addTagToPost = mutation({
  args: {
    postId: v.id("posts"),
    tagId: v.id("tags"),
  },
  handler: async (ctx, { postId, tagId }) => {
    const table = getMutationTable(ctx);
    const post = await table("posts").getX(postId);
    await post.patch({ tags: { add: [tagId] } });
  },
});

export const removeTagFromPost = mutation({
  args: {
    postId: v.id("posts"),
    tagId: v.id("tags"),
  },
  handler: async (ctx, { postId, tagId }) => {
    const table = getMutationTable(ctx);
    const post = await table("posts").getX(postId);
    await post.patch({ tags: { remove: [tagId] } });
  },
});

export const updatePost = mutation({
  args: {
    postId: v.id("posts"),
    title: v.optional(v.string()),
    content: v.optional(v.string()),
    published: v.optional(v.boolean()),
  },
  handler: async (ctx, { postId, ...updates }) => {
    const table = getMutationTable(ctx);
    const post = await table("posts").getX(postId);
    await post.patch(updates);
  },
});

export const deletePost = mutation({
  args: { postId: v.id("posts") },
  handler: async (ctx, { postId }) => {
    const table = getMutationTable(ctx);
    const post = await table("posts").getX(postId);
    await post.delete();
  },
});
