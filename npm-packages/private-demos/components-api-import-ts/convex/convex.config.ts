import { defineApp } from "convex/server";
import waitlist from "../examples/waitlist@name-with-dashes/convex.config.js";

const app = defineApp();
app.use(waitlist, { name: "waitlist" });
app.use(waitlist, { name: "waitlist2" });

export default app;
