// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { defineComponent } from "convex/server";
import httpGrandchild from "../httpGrandchild/convex.config";

const component = defineComponent("httpComponent");
component.use(httpGrandchild, { name: "httpGrandchild", httpPrefix: "/gc/" });
export default component;
