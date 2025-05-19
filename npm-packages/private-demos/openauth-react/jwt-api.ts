import { createClient } from "@openauthjs/openauth/client";
import { subjects } from "./subjects";

const headers = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "*",
  "Access-Control-Allow-Methods": "*",
};

const client = createClient({
  clientID: "jwt-api",
  issuer: "http://localhost:3000",
});

const server = Bun.serve({
  port: 3001,
  async fetch(req) {
    const url = new URL(req.url);

    if (req.method === "OPTIONS") {
      return new Response(null, { headers });
    }

    if (url.pathname === "/" && req.method === "GET") {
      const authHeader = req.headers.get("Authorization");

      if (!authHeader) {
        return new Response("401", { headers, status: 401 });
      }

      const token = authHeader.split(" ")[1];
      const verified = await client.verify(subjects, token);

      if (verified.err) {
        return new Response("401", { headers, status: 401 });
      }

      return new Response(verified.subject.properties.id, { headers });
    }

    return new Response("404", { status: 404 });
  },
});

console.log(`Listening on ${server.url}`);
