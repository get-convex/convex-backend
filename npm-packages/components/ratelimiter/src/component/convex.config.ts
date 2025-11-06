/*
import { ComponentDefinition } from "convex/server";
import { v } from "convex/values";
import { api } from "./_generated/api";
import type { ComponentInterface } from "./_generated/componentInterface";

const component = new ComponentDefinition("ratelimiter", {
  maxLength: v.number(),
});
component.defineExports({
  doSomething: api.index.join,
});

export default component.build<ComponentInterface>();
*/

import { defineComponent } from "convex/server";

export default defineComponent("ratelimiter");
