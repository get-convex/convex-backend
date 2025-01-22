import { defineApp } from "convex/server";
import waitlist from "../examples/waitlist@name-with-dashes/convex.config.js";
import ratelimiter from "@convex-dev/ratelimiter/convex.config.js";
import triggers from "@convex-dev/triggers/convex.config.js";
import waitlistasdf from "../../../components/triggers/src/triggers/convex.config.js";
console.log(waitlistasdf);

const app = defineApp();
app.use(triggers);
app.use(waitlist, { name: "waitlist" });
app.use(waitlist, { name: "waitlist2" });
app.use(ratelimiter);

export default app;
