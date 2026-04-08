import { httpActionGeneric } from "convex/server";
import type { HttpRouter } from "convex/server";

export function registerRoutes(http: HttpRouter) {
  http.route({
    path: "/greet",
    method: "GET",
    handler: httpActionGeneric(
      async () =>
        new Response("hello from legacy component route", { status: 200 }),
    ),
  });

  http.route({
    path: "/echo",
    method: "POST",
    handler: httpActionGeneric(async (_ctx, request) => {
      const body = await request.text();
      return new Response(body, {
        status: 200,
        headers: { "Content-Type": "text/plain" },
      });
    }),
  });
}
