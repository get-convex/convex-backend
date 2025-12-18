import { defineApp } from "convex/server";
import waitlist from "../examples/waitlist@name-with-dashes/convex.config.js";
import nestedComponent from "./nested-component/convex.config.js";

const app = defineApp();
app.use(waitlist, { name: "waitlist" });
app.use(waitlist, { name: "waitlist2" });
app.use(nestedComponent);

export default app;
