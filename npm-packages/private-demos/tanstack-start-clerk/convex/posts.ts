import { action, internalMutation, query } from './_generated/server'
import { api, internal } from './_generated/api.js'
import { v } from 'convex/values'
import { WithoutSystemFields } from 'convex/server'
import { Doc } from './_generated/dataModel'

export const get = query({
  args: {
    postId: v.string(),
  },
  handler: async (ctx, { postId }) => {
    return await ctx.db
      .query('posts')
      .withIndex('id', (q) => q.eq('id', postId))
      .unique()
  },
})

export const list = query(async (ctx) => {
  return await ctx.db.query('posts').collect()
})

export const insert = internalMutation(
  (ctx, { post }: { post: WithoutSystemFields<Doc<'posts'>> }) =>
    ctx.db.insert('posts', post),
)

export const profile = query((ctx) => ctx.auth.getUserIdentity())

export const name = query(
  async (ctx) => (await ctx.auth.getUserIdentity())?.name,
)
export const email = query(
  async (ctx) => (await ctx.auth.getUserIdentity())?.email,
)
export const count = query(
  async (ctx) => (await ctx.db.query('posts').collect()).length,
)

export const populate = action(async (ctx) => {
  const existing = await ctx.runQuery(api.posts.list)
  if (existing.length) {
    return
  }
  const posts = (await (
    await fetch('https://jsonplaceholder.typicode.com/posts')
  ).json()) as {
    userId: string
    id: number
    title: string
    body: string
  }[]
  await Promise.all(
    posts.map((post) =>
      ctx.runMutation(internal.posts.insert, {
        post: {
          id: post.id.toString(),
          body: post.body,
          title: post.title,
        },
      }),
    ),
  )
})
