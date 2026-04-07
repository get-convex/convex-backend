import { httpAction } from "./_generated/server";
import { httpRouter } from "convex/server";

const http = httpRouter();

http.route({
  path: "/test",
  method: "POST",
  handler: httpAction(async (_) => {
    return new Response(null, {});
  }),
});

export default http;
