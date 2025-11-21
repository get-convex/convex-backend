import { Doc } from "../../../component/_generated/dataModel";
import { query, action, mutation } from "./_generated/server";
import { components } from "./_generated/api";

export const list = query({
  args: {},
  handler: async (ctx): Promise<Doc<"messages">[]> => {
    const result = await ctx.runQuery(
      components.component.messages.listMessages,
      {},
    );
    console.log(result);
    return result;
  },
});

export const insert = mutation({
  handler: async (
    ctx,
    { channel, text }: { channel: string; text: string },
  ) => {
    await ctx.runMutation(components.component.messages.insertMessage, {
      channel,
      text,
    });
  },
});

export const hello = action({
  args: {},
  handler: async (ctx) => {
    return await ctx.runAction(components.component.messages.hello, {});
  },
});
