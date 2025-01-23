// This is an MJS module because of its file extension .mts
// but `esbuild` can compile it to either format.

import { ConvexHttpClient } from "convex/browser";

import { api } from "../convex/_generated/api_cjs.cjs";

const client = new ConvexHttpClient("http://127.0.0.1:8000");

// eslint-disable-next-line  @typescript-eslint/no-unused-vars
async function doSomething() {
  void (await client.query(api.getSomething.default));
}
