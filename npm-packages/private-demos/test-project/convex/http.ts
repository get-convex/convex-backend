import { httpRouter } from "convex/server";
import { auth } from "./auth";
import { httpAction } from "./_generated/server";

const http = httpRouter();

auth.addHttpRoutes(http);

http.route({
  path: "/",
  method: "GET",
  handler: httpAction(async (_ctx, _request) => {
    return new Response(`Hello world!`);
  }),
});

export default http;
