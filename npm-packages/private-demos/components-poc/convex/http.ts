import { myHttpRoute, registerRoutes } from "@convex-dev/ratelimiter";
import { httpRouter } from "convex/server";

const http = httpRouter();

// Routes defined in clients can be routd directly...
http.route({
  path: "/ratelimiter-route",
  method: "GET",
  handler: myHttpRoute,
});

// ...or added in batch.
registerRoutes(http);

export default http;
