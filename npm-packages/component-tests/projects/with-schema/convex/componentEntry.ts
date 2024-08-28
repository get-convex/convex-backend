import { Doc } from "../../../component/_generated/dataModel";
import { query, components, action, mutation } from "./_generated/server";

export const list = query(async (ctx): Promise<Doc<"messages">[]> => {
  const result = await ctx.runQuery(
    components.component.messages.listMessages,
    {},
  );
  console.log(result);
  return result;
});

export const insert = mutation(
  async (ctx, { channel, text }: { channel: string; text: string }) => {
    await ctx.runMutation(components.component.messages.insertMessage, {
      channel,
      text,
    });
  },
);

export const hello = action(async (ctx) => {
  return await ctx.runAction(components.component.messages.hello, {});
});
