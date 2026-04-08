import { httpRouter } from "convex/server";
import { httpAction } from "./_generated/server";

const http = httpRouter();

http.route({
  pathPrefix: "/",
  method: "GET",
  handler: httpAction(async () => {
    return new Response("app custom 404", { status: 404 });
  }),
});

export default http;
