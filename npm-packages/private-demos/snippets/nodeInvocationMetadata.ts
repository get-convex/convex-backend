import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api";

const client = new ConvexHttpClient(process.env["CONVEX_URL"]);

await client.action(
  api.invocationMetadata.processOrder,
  { orderId: "order_123" },
  {
    metadata: {
      correlationId: "req_123",
      origin: "checkout",
    },
  },
);
