import { httpRouter } from "convex/server";
import { httpAction } from "./_generated/server";
import { api } from "./_generated/api";

const http = httpRouter();

http.route({
  path: "/pausableHello",
  method: "GET",
  handler: httpAction(async (_ctx, request) => {
    const waitForAbort = new Promise((resolve) => {
      request.signal.addEventListener("abort", () => {
        console.log("Abort event received");
        resolve(null);
      });
    });
    // Test pauses here and disconnects the client.
    await new Promise((resolve) => setTimeout(resolve, 0));
    // Wait for client to disconnect.
    await waitForAbort;
    return new Response("Hello", {
      status: 200,
      headers: {
        "Content-Type": "text/plain",
      },
    });
  }),
});

http.route({
  path: "/pausableHelloBody",
  method: "GET",
  handler: httpAction(async (_ctx, request) => {
    const waitForAbort = new Promise((resolve) => {
      request.signal.addEventListener("abort", () => {
        console.log("Abort event received");
        resolve(null);
      });
    });
    const encoder = new TextEncoder();
    const body = new ReadableStream({
      async start(controller) {
        controller.enqueue(encoder.encode("Hello, "));
        // Test pauses here and disconnects the client.
        await new Promise((resolve) => setTimeout(resolve, 0));
        await waitForAbort;
        controller.enqueue(encoder.encode("World"));
        controller.close();
      },
    });
    return new Response(body, {
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

http.route({
  path: "/writeAfterDisconnect",
  method: "GET",
  handler: httpAction(async (ctx, request) => {
    await new Promise((resolve) => setTimeout(resolve, 0));
    return new Promise((resolve) => {
      request.signal.addEventListener("abort", async () => {
        await ctx.runMutation(api.functions.write, {});
        console.log("Abort event received");
        resolve(new Response("Hello, world!"));
      });
    });
  }),
});

export default http;
