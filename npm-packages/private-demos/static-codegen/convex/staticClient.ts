import type { ComponentApi } from "../examples/static-component/_generated/component.js";
import { ActionCtx } from "./_generated/server.js";

export async function callComponentFunctions(
  ctx: ActionCtx,
  component: ComponentApi,
) {
  const branded = await ctx.runQuery(component.staticFunctions.q, {
    branded: "1",
  });
  const result = await ctx.runMutation(component.staticFunctions.m, {
    branded,
  });
  if (!result) {
    throw new Error("Failed to insert document");
  }
  await ctx.runAction(component.staticFunctions.a, {
    branded,
    id: result._id,
  });
}
