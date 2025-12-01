import { v } from "convex/values";
import { mutation, query, getMutationTable, getQueryTable } from "./functions";

export const createComment = mutation({
  args: {
    postId: v.id("posts"),
    authorId: v.id("users"),
    text: v.string(),
  },
  handler: async (ctx, args) => {
    const table = getMutationTable(ctx);
    const commentId = await table("comments").insert({
      ...args,
      createdAt: Date.now(),
    });
    return commentId;
  },
});

export const getCommentsByPost = query({
  args: { postId: v.id("posts") },
  handler: async (ctx, { postId }) => {
    const table = getQueryTable(ctx);
    const post = await table("posts").getX(postId);
    const comments = await post.edge("comments");

    return await Promise.all(
      comments.map(async (comment) => {
        const author = await comment.edge("author");
        return {
          _id: comment._id,
          text: comment.text,
          createdAt: comment.createdAt,
          author: {
            _id: author._id,
            name: author.name,
          },
        };
      }),
    );
  },
});

export const deleteComment = mutation({
  args: { commentId: v.id("comments") },
  handler: async (ctx, { commentId }) => {
    const table = getMutationTable(ctx);
    const comment = await table("comments").getX(commentId);
    await comment.delete();
  },
});
