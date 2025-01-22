const { ConvexHttpClient } = require("convex/browser");
const { api } = require("./convex/_generated/api_cjs.cjs");
require("dotenv").config({ path: ".env.local" });
const client = new ConvexHttpClient(process.env["CONVEX_URL"]);

client.query(api.tasks.get).then(console.log);
