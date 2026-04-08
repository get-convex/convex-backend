import { httpRouter } from "convex/server";
import { httpAction } from "./_generated/server";
import { components } from "./_generated/api";

const http = httpRouter();

http.route({
  path: "/hello",
  method: "GET",
  handler: httpAction(async () => {
    return new Response("hello from component", { status: 200 });
  }),
});

http.route({
  path: "/site-url",
  method: "GET",
  handler: httpAction(async () => {
    const siteUrl = process.env.CONVEX_SITE_URL ?? "not set";
    return new Response(siteUrl, { status: 200 });
  }),
});

http.route({
  path: "/grandchild-greeting",
  method: "GET",
  handler: httpAction(async (ctx) => {
    const greeting = await ctx.runQuery(
      components.httpGrandchild.public.greeting,
      {},
    );
    return new Response(greeting, { status: 200 });
  }),
});

export default http;
