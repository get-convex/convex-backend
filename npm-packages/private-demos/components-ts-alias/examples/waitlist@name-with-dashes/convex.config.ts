import { defineComponent } from "convex/server";
import ratelimiter from "@convex-dev/ratelimiter/convex.config";

const componentDefinition = defineComponent("waitlist");
componentDefinition.use(ratelimiter);
export default componentDefinition;
