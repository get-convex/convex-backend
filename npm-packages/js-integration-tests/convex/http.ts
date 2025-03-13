import { httpRouter, UserIdentity } from "convex/server";
import { httpAction } from "./_generated/server";
import { ConvexError } from "convex/values";

const http = httpRouter();

http.route({
  path: "/sendImage",
  method: "POST",
  handler: httpAction(async ({ storage }, request) => {
    const blob = await request.blob();
    const digestHeader = request.headers.get("digest");
    const sha256 =
      digestHeader !== null && digestHeader.startsWith("sha-256=")
        ? digestHeader.slice(8)
        : undefined;
    const storageId = await storage.store(blob, {
      sha256: sha256,
    });

    return new Response(JSON.stringify({ storageId }), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  }),
});

http.route({
  path: "/deleteImage",
  method: "POST",
  handler: httpAction(async ({ storage }, request) => {
    const storageId = new URL(request.url).searchParams.get("storageId")!;
    await storage.delete(storageId);

    return new Response(null, {
      status: 200,
    });
  }),
});

http.route({
  path: "/getImage",
  method: "GET",
  handler: httpAction(async ({ storage }, request) => {
    const storageId = new URL(request.url).searchParams.get("storageId")!;
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
    const storageId = new URL(request.url).searchParams.get("storageId")!;
    const imageUrl = await storage.getUrl(storageId);
    if (imageUrl === null) {
      return new Response("Image not found", {
        status: 404,
      });
    }
    return Response.redirect(imageUrl);
  }),
});

http.route({
  path: "/authHeader",
  method: "GET",
  handler: httpAction(async ({ auth }, request) => {
    let identity: UserIdentity | null | "error" = null;
    try {
      identity = await auth.getUserIdentity();
    } catch (e) {
      identity = "error";
    }

    const result = {
      identity,
      authorizationHeader: request.headers.get("Authorization"),
    };
    return new Response(JSON.stringify(result));
  }),
});

http.route({
  path: "/failer",
  method: "GET",
  handler: httpAction(async (_ctx, _request) => {
    throw new Error("ErrMsg");
  }),
});

http.route({
  path: "/failer_custom",
  method: "GET",
  handler: httpAction(async (_ctx, _request) => {
    throw new ConvexError("Hello world!");
  }),
});

http.route({
  path: "/",
  method: "POST",
  handler: httpAction(async (_ctx, request) => {
    const text = await request.text();
    return new Response(text, { status: 200 });
  }),
});

http.route({
  path: "/getMetadata",
  method: "GET",
  handler: httpAction(async ({ storage }, request) => {
    const storageId = new URL(request.url).searchParams.get("storageId")!;
    const res = await storage.getMetadata(storageId);

    if (res === null) {
      return new Response("File not found", {
        status: 404,
      });
    }

    return new Response(JSON.stringify(res));
  }),
});

export default http;
