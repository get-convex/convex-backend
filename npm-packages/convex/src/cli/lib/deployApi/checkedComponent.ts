import { z } from "zod";
import {
  componentDefinitionPath,
  componentFunctionPath,
  ComponentDefinitionPath,
  ComponentPath,
  componentPath,
} from "./paths.js";
import { Identifier, identifier } from "./types.js";

export const resource = z.union([
  z.object({ type: z.literal("value"), value: z.string() }),
  z.object({ type: z.literal("function"), path: componentFunctionPath }),
]);
export type Resource = z.infer<typeof resource>;

export type CheckedExport =
  | { type: "branch"; children: Record<Identifier, CheckedExport> }
  | { type: "leaf"; resource: Resource };
export const checkedExport: z.ZodType<CheckedExport> = z.lazy(() =>
  z.union([
    z.object({
      type: z.literal("branch"),
      children: z.record(identifier, checkedExport),
    }),
    z.object({
      type: z.literal("leaf"),
      resource,
    }),
  ]),
);

export const checkedHttpRoutes = z.object({
  routerPrefix: z.array(z.tuple([z.string(), z.string()])),
  routerExact: z.array(z.tuple([z.string(), z.string()])),
  mountedPrefix: z.array(z.string()),
});
export type CheckedHttpRoutes = z.infer<typeof checkedHttpRoutes>;

export type CheckedComponent = {
  definitionPath: ComponentDefinitionPath;
  componentPath: ComponentPath;
  args: Record<Identifier, Resource>;
  childComponents: Record<Identifier, CheckedComponent>;
};
export const checkedComponent: z.ZodType<CheckedComponent> = z.lazy(() =>
  z.object({
    definitionPath: componentDefinitionPath,
    componentPath,
    args: z.record(identifier, resource),
    childComponents: z.record(identifier, checkedComponent),
    httpRoutes: checkedHttpRoutes,
    exports: z.record(identifier, checkedExport),
  }),
);
