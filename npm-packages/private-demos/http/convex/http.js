import { httpRouter } from "convex/server";
import checkUrl from "./checkUrl";
import getMessagesByAuthor from "./getMessagesByAuthor";
import { httpAction } from "./_generated/server";
import { api } from "./_generated/api";

const http = httpRouter();

http.route({
  path: "/postMessage",
  method: "POST",
  handler: httpAction(async ({ runMutation }, request) => {
    const { author, body } = await request.json();

    await runMutation(api.sendMessage.default, { body, author });
    return new Response(null, {
      status: 200,
    });
  }),
});

http.route({
  path: "/getMessagesByAuthor",
  method: "GET",
  handler: getMessagesByAuthor,
});

http.route({
  path: "/checkUrl",
  method: "GET",
  handler: checkUrl,
});

http.route({
  pathPrefix: "/prefix/",
  method: "GET",
  handler: httpAction(async (_, request) => {
    const url = new URL(request.url);
    return new Response(url.pathname, {
      headers: {
        "content-type": "text/plain",
      },
      status: 200,
    });
  }),
});

for (const method of ["GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH"]) {
  http.route({
    path: "/method-test",
    method,
    handler: httpAction((_ctx, _request) => {
      return new Response(method, { status: 200 });
    }),
  });
}

http.route({
  path: "/sendImage",
  method: "POST",
  handler: httpAction(async ({ storage, runMutation }, request) => {
    // Could check auth and 401 / 403 if we wanted
    const blob = await request.blob();
    const storageId = await storage.store(blob).catch((reason) => {
      new Response(reason);
    });
    const author = new URL(request.url).searchParams.get("author");

    const headers = new Headers({
      "Access-Control-Allow-Credentials": "true",
      "Access-Control-Allow-Origin": "http://localhost:3000",
      "Access-Control-Allow-Methods": "POST,OPTIONS",
      "Access-Control-Max-Age": "86400",
      "Access-Control-Allow-Headers": request.headers.get(
        "Access-Control-Request-Headers",
      ),
    });

    await runMutation(api.sendMessage.sendImage, { storageId, author });
    return new Response(null, {
      status: 200,
      headers,
    });
  }),
});

http.route({
  path: "/deleteImage",
  method: "POST",
  handler: httpAction(async ({ storage }, request) => {
    const storageId = new URL(request.url).searchParams.get("storageId");
    await storage.delete(storageId);

    const headers = new Headers({
      "Access-Control-Allow-Credentials": "true",
      "Access-Control-Allow-Origin": "http://localhost:3000",
      "Access-Control-Allow-Methods": "POST,OPTIONS",
      "Access-Control-Max-Age": "86400",
      "Access-Control-Allow-Headers": request.headers.get(
        "Access-Control-Request-Headers",
      ),
    });

    return new Response(null, {
      status: 200,
      headers,
    });
  }),
});

http.route({
  pathPrefix: "/",
  method: "OPTIONS",
  handler: httpAction(async (_, request) => {
    const corsHeaders = {
      "Access-Control-Allow-Credentials": "true",
      "Access-Control-Allow-Origin": "http://localhost:3000",
      "Access-Control-Allow-Methods": "POST,OPTIONS,GET",
      "Access-Control-Max-Age": "86400",
    };

    // Make sure the necessary headers are present
    // for this to be a valid pre-flight request
    let headers = request.headers;
    if (
      headers.get("Origin") !== null &&
      headers.get("Access-Control-Request-Method") !== null &&
      headers.get("Access-Control-Request-Headers") !== null
    ) {
      // Handle CORS pre-flight request.
      // If you want to check or reject the requested method + headers
      // you can do that here.
      let respHeaders = {
        ...corsHeaders,
        // Allow all future content Request headers to go back to browser
        // such as Authorization (Bearer) or X-Client-Name-Version
        "Access-Control-Allow-Headers": request.headers.get(
          "Access-Control-Request-Headers",
        ),
      };
      return new Response(null, {
        headers: respHeaders,
      });
    } else {
      // Handle standard OPTIONS request.
      // If you want to allow other HTTP Methods, you can do that here.
      return new Response(null, {
        headers: {
          Allow: "POST, OPTIONS, GET",
        },
      });
    }
  }),
});

http.route({
  path: "/getImage",
  method: "GET",
  handler: httpAction(async ({ storage }, request) => {
    // Could check auth and 401 / 403 if we wanted
    const storageId = new URL(request.url).searchParams.get("storageId");
    const blob = await storage.get(storageId);
    if (blob === null) {
      return new Response("Image not found", {
        status: 404,
      });
    }
    return new Response(blob, {
      headers: { "content-type": blob.type },
    });
  }),
});

http.route({
  path: "/getImageWithRedirect",
  method: "GET",
  handler: httpAction(async ({ storage }, request) => {
    // Could check auth and 401 / 403 if we wanted
    const storageId = new URL(request.url).searchParams.get("storageId");
    // These could be time expired URLs if we wanted
    const imageUrl = await storage.getUrl(storageId);
    return Response.redirect(imageUrl);
  }),
});

http.route({
  path: "/longRunningAction",
  method: "GET",
  handler: httpAction(async ({ runAction }, _request) => {
    const result = await runAction(api.actions.longRunning);
    return new Response(result);
  }),
});

export default http;
