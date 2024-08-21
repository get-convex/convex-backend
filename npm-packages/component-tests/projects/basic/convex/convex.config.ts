// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { defineApp } from "convex/server";
import envVars from "../../../envVars/convex.config";
import errors from "../../../errors/convex.config";

// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
const app = defineApp();

app.install(errors, { args: {} });
app.install(envVars, {
  args: { name: process.env.NAME, url: process.env.CONVEX_CLOUD_URL },
});

export default app;
