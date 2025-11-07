import { defineApp } from "convex/server";
import waitlist from "@convex-dev/waitlist/convex.config";

const app = defineApp();
app.use(waitlist, { name: "waitlist" });
app.use(waitlist, { name: "waitlist2" });

export default app;
