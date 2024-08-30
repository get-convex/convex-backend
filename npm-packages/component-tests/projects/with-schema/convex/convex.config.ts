// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { defineApp } from "convex/server";
import component from "../../../component/convex.config";

// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
const app = defineApp();

app.use(component);

export default app;
