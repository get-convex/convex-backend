import { ConvexClient } from "convex/browser";
import dns from "dns";

dns.setDefaultResultOrder("ipv4first");
const deploymentUrl = process.env.DEPLOYMENT_URL || "http://127.0.0.1:8000";

let client = new ConvexClient(deploymentUrl);
const result = await client.query("getUsers");
console.log(result);
await new Promise((r) => setTimeout(r, 100));
await client.close();
