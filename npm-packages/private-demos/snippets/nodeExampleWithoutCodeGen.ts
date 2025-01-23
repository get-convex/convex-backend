import { ConvexHttpClient } from "convex/browser";
import { anyApi } from "convex/server";

const client = new ConvexHttpClient(process.env["CONVEX_URL"]);

// either this
const count = await client.query(anyApi.counter.get);
// or this
client.query(anyApi.counter.get).then((count) => console.log(count));
