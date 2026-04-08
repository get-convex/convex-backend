import { httpRouter } from "convex/server";
import { httpAction } from "./_generated/server";

const http = httpRouter();

http.route({
  path: "/echo",
  method: "POST",
  handler: httpAction(async (_ctx, request) => {
    const body = await request.text();
    return new Response(body, {
      status: 200,
      headers: { "Content-Type": "text/plain" },
    });
  }),
});

export default http;
