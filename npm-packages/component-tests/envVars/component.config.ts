// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { defineComponent } from "convex/server";
import { v } from "convex/values";

export default defineComponent("envVars", {
  args: { name: v.optional(v.string()), url: v.optional(v.string()) },
});
