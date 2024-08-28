// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { defineApp } from "convex/server";
import component from "../../../component/convex.config";
import envVars from "../../../envVars/convex.config";

// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
const app = defineApp();

const c = app.install(component);
app.install(envVars);
app.mount({ mounted: c.exports });

export default app;
