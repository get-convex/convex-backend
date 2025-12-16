import { httpRouter } from "convex/server";
import { httpAction } from "./_generated/server";

const http = httpRouter();

http.route({
  path: "/some/path",
  method: "POST",
  handler: httpAction(async (_ctx, _request) => {
    return new Response(`Hello, world!`);
  }),
});
export default http;
