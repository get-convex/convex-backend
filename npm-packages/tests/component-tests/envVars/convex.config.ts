// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { defineComponent } from "convex/server";
import { default as otherComponent } from "../component/convex.config";

const component = defineComponent("envVars");
component.use(otherComponent, {
  name: "component",
});

export default component;
