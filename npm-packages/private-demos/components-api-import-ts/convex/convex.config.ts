import { defineApp } from "convex/server";
import waitlist from "../examples/waitlist@name-with-dashes/convex.config.js";
import nested from "./nested-component/convex.config.js";

const app = defineApp();
app.use(waitlist, { name: "waitlist" });
app.use(waitlist, { name: "waitlist2" });
app.use(nested);

export default app;
