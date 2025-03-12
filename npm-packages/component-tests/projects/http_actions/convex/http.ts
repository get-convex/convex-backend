import { httpRouter } from "convex/server";
import { httpAction } from "./_generated/server";

const http = httpRouter();

http.route({
  path: "/pausableHello",
  method: "GET",
  handler: httpAction(async (_ctx, _request) => {
    // Test can pause on this sleep.
    await new Promise((resolve) => setTimeout(resolve, 0));
    return new Response("Hello", {
      status: 200,
      headers: {
        "Content-Type": "text/plain",
      },
    });
  }),
});

http.route({
  path: "/echo",
  method: "POST",
  handler: httpAction(async (_ctx, request) => {
    const body = await request.text();
    return new Response(body, {
      status: 200,
      headers: {
        "Content-Type": "text/plain",
      },
    });
  }),
});

http.route({
  path: "/errorInEndpoint",
  method: "GET",
  handler: httpAction(async (_ctx, _request) => {
    throw new Error("Custom error");
  }),
});

export default http;
