import { defineApp } from "convex/server";
import counterComponent from "../counterComponent/convex.config.js";

const app = defineApp();
app.use(counterComponent);
export default app;
