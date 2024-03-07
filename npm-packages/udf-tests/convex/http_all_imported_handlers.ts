import { httpRouter } from "convex/server";
import { imported } from "./http_no_default";

const http = httpRouter();

http.route({
  path: "/test1",
  method: "POST",
  handler: imported,
});

http.route({
  path: "/test2",
  method: "POST",
  handler: imported,
});

http.route({
  path: "/test3",
  method: "POST",
  handler: imported,
});

http.route({
  path: "/test4",
  method: "POST",
  handler: imported,
});

export default http;
