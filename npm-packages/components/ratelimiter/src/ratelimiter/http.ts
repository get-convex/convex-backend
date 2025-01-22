import { httpRouter } from "convex/server";
import { httpAction, internalQuery } from "./_generated/server.js";
import { internal } from "./_generated/api.js";

const http = httpRouter();
http.route({
  path: "/index.html",
  method: "GET",
  handler: httpAction(async (ctx) => {
    const docs = await ctx.runQuery(internal.http.loadDebug, {});
    const content = docs.map((doc) => JSON.stringify(doc, null, 2)).join("\n");
    return new Response(`<html><body><pre>${content}</pre></body></html>`, {
      headers: {
        "Content-Type": "text/html",
      },
    });
  }),
});

export const loadDebug = internalQuery({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("rateLimits").collect();
  },
});

export default http;
