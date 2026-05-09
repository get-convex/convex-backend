import { defineApp } from "convex/server";
import { v } from "convex/values";
import component from "../fakecomponent/convex.config";

const app = defineApp({
  env: {
    GREETING: v.optional(v.union(v.literal("yo"), v.literal("hey"))),
    MESSAGE: v.string(),
  },
});

app.use(component, { env: { THING_I_NEED: "a string" } });

export default app;
