import { defineApp } from "convex/server";
import httpComponent from "../../../httpComponent/convex.config";

const app = defineApp();

app.use(httpComponent); // no httpPrefix — component HTTP routes should not be accessible

export default app;
