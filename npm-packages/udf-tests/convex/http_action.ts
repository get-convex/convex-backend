import { httpRouter } from "convex/server";
import { imported } from "./http_no_default";
import { api } from "./_generated/api";
import { httpAction, query } from "./_generated/server";

const basic = httpAction(
  async ({ runQuery, runMutation, runAction }, request: Request) => {
    const countBefore = await runQuery(api.basic.count);
    const createdObject = await runMutation(api.basic.insertObject, {
      foo: BigInt(42),
    });
    const countAfter = await runQuery(api.basic.count);

    const actionResult = await runAction(api.basic.simpleAction, {});
    // TODO: This should really be returning a Response object, but
    // for now just return some JSON so we can assert the result matches
    return new Response(
      JSON.stringify({
        requestBody: await request.text(),
        countBefore,
        countAfter,
        actionResult,
        // Check that we correctly deserialize the return values
        // from `runMutation` etc.
        isBigInt: typeof createdObject!.foo === "bigint",
      }),
    );
  },
);

// Regression test ensuring blob can be passed from fetch Response to action Response.
const proxyFetchResponse = httpAction(async (_ctx, _request) => {
  return await fetch("http://localhost:4545/echo_server", {
    method: "POST",
    body: "Hello World",
  });
});

// Round trip Request -> fetch echo -> Response,
// with different ways of passing the body through.
const roundTripFetchBlob = httpAction(async (_ctx, request) => {
  const response = await fetch("http://localhost:4545/echo_server", {
    method: request.method,
    body: await request.blob(),
  });
  return new Response(await response.blob());
});
const roundTripFetchText = httpAction(async (_ctx, request) => {
  const response = await fetch("http://localhost:4545/echo_server", {
    method: request.method,
    body: await request.text(),
  });
  return new Response(await response.text());
});
const roundTripFetchArrayBuffer = httpAction(async (_ctx, request) => {
  const response = await fetch("http://localhost:4545/echo_server", {
    method: request.method,
    body: await request.arrayBuffer(),
  });
  return new Response(await response.arrayBuffer());
});
const roundTripFetchJson = httpAction(async (_ctx, request) => {
  const response = await fetch("http://localhost:4545/echo_server", {
    method: request.method,
    body: JSON.stringify(await request.json()),
  });
  return new Response(JSON.stringify(await response.json()));
});

const echo = httpAction(async (_ctx, request) => {
  return new Response(await request.blob());
});

const schedule = httpAction(async ({ scheduler }, _request) => {
  await scheduler.runAfter(2000, api.basic.insertObject, { foo: "bar" });
  return new Response();
});

const streamResponse = httpAction(async (_ctx, _request) => {
  const encoder = new TextEncoder();
  const stream = new ReadableStream({
    type: "bytes",
    start(controller) {
      controller.enqueue(encoder.encode("<html>"));
      setTimeout(() => {
        controller.enqueue(encoder.encode("</html>"));
        controller.close();
      }, 20);
    },
  });
  return new Response(stream);
});

const streamDanglingResponse = httpAction(async (_ctx, _request) => {
  const encoder = new TextEncoder();
  const stream = new ReadableStream({
    type: "bytes",
    start(controller) {
      controller.enqueue(encoder.encode("<html>"));
    },
  });
  return new Response(stream);
});

const slowResponse = httpAction(async (_ctx, _request) => {
  await new Promise((resolve) => setTimeout(resolve, 900));
  return new Response("slow");
});

const http = httpRouter();
http.route({
  method: "POST",
  path: "/basic",
  handler: basic,
});
http.route({
  method: "GET",
  path: "/proxy_response",
  handler: proxyFetchResponse,
});
http.route({
  method: "POST",
  path: "/round_trip_fetch_blob",
  handler: roundTripFetchBlob,
});
http.route({
  method: "POST",
  path: "/round_trip_fetch_text",
  handler: roundTripFetchText,
});
http.route({
  method: "POST",
  path: "/round_trip_fetch_array_buffer",
  handler: roundTripFetchArrayBuffer,
});
http.route({
  method: "POST",
  path: "/round_trip_fetch_json",
  handler: roundTripFetchJson,
});
http.route({
  method: "POST",
  path: "/echo",
  handler: echo,
});
http.route({
  method: "GET",
  path: "/schedule",
  handler: schedule,
});
http.route({
  method: "GET",
  path: "/stream_response",
  handler: streamResponse,
});
http.route({
  method: "GET",
  path: "/stream_dangling_response",
  handler: streamDanglingResponse,
});
http.route({
  method: "GET",
  path: "/slow",
  handler: slowResponse,
});
http.route({
  method: "GET",
  path: "/errorInRun",
  handler: httpAction(async ({ runQuery }, _request: Request) => {
    await runQuery(api.http_action.erroringQuery);
    return new Response("success");
  }),
});

http.route({
  method: "GET",
  path: "/errorInRunCatch",
  handler: httpAction(async ({ runQuery }, _request: Request) => {
    try {
      await runQuery(api.http_action.erroringQuery);
    } catch {
      // do nothing
    }
    return new Response("success");
  }),
});

http.route({
  method: "GET",
  path: "/errorInEndpoint",
  handler: httpAction(async (_, _request: Request) => {
    throw new Error("Oh no!");
  }),
});

http.route({
  method: "GET",
  path: "/imported",
  handler: imported,
});

http.route({
  method: "GET",
  path: "/convexCloudSystemVar",
  handler: httpAction(async (_, _request: Request) => {
    return new Response(process.env.CONVEX_CLOUD_URL);
  }),
});

http.route({
  method: "GET",
  path: "/convexSiteSystemVar",
  handler: httpAction(async (_, _request: Request) => {
    return new Response(process.env.CONVEX_SITE_URL);
  }),
});

http.route({
  method: "POST",
  path: "/largeResponse",
  handler: httpAction(async (_, request: Request) => {
    const size = +(await request.text());
    const stringLength = size * 1024 * 1024;
    return new Response("a".repeat(stringLength));
  }),
});

export const erroringQuery = query(() => {
  throw new Error("Oh no! Called erroring query");
});

export default http;
