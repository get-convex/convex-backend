// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { defineApp } from "convex/server";
import component from "../component/component.config";

// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
const app = defineApp();

app.install(component, {
  args: { name: process.env.NAME, url: process.env.CONVEX_CLOUD_URL },
});

export default app;
