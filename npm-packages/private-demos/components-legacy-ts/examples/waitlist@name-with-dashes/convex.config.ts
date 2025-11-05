import { defineComponent } from "convex/server";
import ratelimiter from "@convex-dev/ratelimiter/convex.config.js";

const componentDefinition = defineComponent("waitlist");
componentDefinition.use(ratelimiter);
export default componentDefinition;
