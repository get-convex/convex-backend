// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { defineComponent } from "convex/server";
import { v } from "convex/values";
import { default as otherComponent } from "../component/convex.config";

const component = defineComponent("envVars", {
  args: { name: v.optional(v.string()), url: v.optional(v.string()) },
});
component.installWithInit(otherComponent, {
  name: "component",
  onInit: (_ctx, args) => {
    const name = args.name ?? "a nice default name";
    const url = args.url ?? "https://carnitas.convex.cloud";
    return { name, url };
  },
});

export default component;
