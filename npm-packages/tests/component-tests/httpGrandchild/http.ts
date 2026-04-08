import { httpRouter } from "convex/server";
import { httpAction } from "./_generated/server";

const http = httpRouter();

http.route({
  path: "/grandchild-hello",
  method: "GET",
  handler: httpAction(async () => {
    return new Response("hello from grandchild component", { status: 200 });
  }),
});

http.route({
  path: "/site-url",
  method: "GET",
  handler: httpAction(async () => {
    return new Response(process.env.CONVEX_SITE_URL, { status: 200 });
  }),
});

http.route({
  pathPrefix: "/",
  method: "GET",
  handler: httpAction(async () => {
    return new Response("grandchild custom 404", { status: 404 });
  }),
});

export default http;
