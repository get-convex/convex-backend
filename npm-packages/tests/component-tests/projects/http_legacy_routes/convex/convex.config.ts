import { defineApp } from "convex/server";
import httpComponent from "../../../httpComponent/convex.config";
const app = defineApp();
app.use(httpComponent);
export default app;
