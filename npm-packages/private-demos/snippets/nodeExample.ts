import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api";

const client = new ConvexHttpClient(process.env["CONVEX_URL"]);

// either this
const count = await client.query(api.counter.get);
// or this
client.query(api.counter.get).then((count) => console.log(count));
