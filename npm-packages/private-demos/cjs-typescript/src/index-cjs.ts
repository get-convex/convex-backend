// This is a CJS module because package.json does not have `type: "module"` in it
// but `esbuild` can compile it to either format.

import * as dotenv from "dotenv";

import { ConvexHttpClient } from "convex/browser";

import { api } from "../convex/_generated/api_cjs.cjs";

dotenv.config();

const client = new ConvexHttpClient("http://127.0.0.1:8000");

async function doSomething() {
  void (await client.query(api.getSomething.default));
}
console.log(doSomething);
