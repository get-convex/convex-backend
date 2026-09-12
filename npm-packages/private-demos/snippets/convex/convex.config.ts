import { defineApp } from "convex/server";
import { v } from "convex/values";

const app = defineApp({
  env: {
    // Shared secret that callers of `loadOne` must pass as the `token` argument.
    CONVEX_AUTH_TOKEN: v.string(),
  },
});

export default app;
