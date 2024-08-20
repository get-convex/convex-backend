import path from "path";
import { Context } from "../../bundler/context.js";
import { entryPoints } from "../../bundler/index.js";
import {
  ComponentDirectory,
  toComponentDefinitionPath,
} from "../lib/components/definition/directoryStructure.js";
import { StartPushResponse } from "../lib/deployApi/startPush.js";
import { importPath, moduleIdentifier } from "./api.js";
import { header } from "./common.js";
import {
  ComponentExports,
  EvaluatedComponentDefinition,
} from "../lib/deployApi/componentDefinition.js";
import { Identifier } from "../lib/deployApi/types.js";
import { ComponentDefinitionPath } from "../lib/deployApi/paths.js";
import { resolveFunctionReference } from "./component_server.js";

export function componentApiJs(isRoot: boolean) {
  const lines = [];
  if (isRoot) {
    lines.push(header("Generated `api` utility."));
    lines.push(`
      import { anyApi } from "convex/server";

      /**
       * A utility for referencing Convex functions in your app's API.
       *
       * Usage:
       * \`\`\`js
       * const myFunctionReference = api.myModule.myFunction;
       * \`\`\`
       */
      export const api = anyApi;
      export const internal = anyApi;
    `);
  } else {
    lines.push(header("Generated `api` utility."));
    lines.push(`
      import { anyApi } from "convex/server";
      /**
       * A utility for referencing Convex functions in your app's API.
       *
       * Usage:
       * \`\`\`js
       * const myFunctionReference = functions.myModule.myFunction;
       * \`\`\`
       */
      export const functions = anyApi;
    `);
  }
  return lines.join("\n");
}

export function rootComponentApiCJS() {
  const lines = [];
  lines.push(header("Generated `api` utility."));
  lines.push(`const { anyApi } = require("convex/server");`);
  lines.push(`module.exports = {
    api: anyApi,
    internal: anyApi,
  };`);
  return lines.join("\n");
}

export function componentApiStubDTS(isRoot: boolean) {
  const lines = [];
  lines.push(header("Generated `api` utility."));
  lines.push(`import type { AnyApi } from "convex/server";`);
  if (isRoot) {
    lines.push(`
      export declare const api: AnyApi;
      export declare const internal: AnyApi;
    `);
  } else {
    lines.push(`export declare const functions: AnyApi;`);
  }
  return lines.join("\n");
}

export async function componentApiDTS(
  ctx: Context,
  startPush: StartPushResponse,
  rootComponent: ComponentDirectory,
  componentDirectory: ComponentDirectory,
) {
  const isRoot = componentDirectory.isRoot;
  const definitionPath = toComponentDefinitionPath(
    rootComponent,
    componentDirectory,
  );
  const absModulePaths = await entryPoints(ctx, componentDirectory.path, false);
  const modulePaths = absModulePaths.map((p) =>
    path.relative(componentDirectory.path, p),
  );

  const lines = [];
  if (isRoot) {
    lines.push(header("Generated `api` utility."));
    for (const modulePath of modulePaths) {
      const ident = moduleIdentifier(modulePath);
      const path = importPath(modulePath);
      lines.push(`import type * as ${ident} from "../${path}.js";`);
    }
    lines.push(`
      import type {
        ApiFromModules,
        FilterApi,
        FunctionReference,
      } from "convex/server";
      /**
       * A utility for referencing Convex functions in your app's API.
       *
       * Usage:
       * \`\`\`js
       * const myFunctionReference = api.myModule.myFunction;
       * \`\`\`
       */
      declare const fullApi: ApiFromModules<{
    `);
    for (const modulePath of modulePaths) {
      const ident = moduleIdentifier(modulePath);
      const path = importPath(modulePath);
      lines.push(`  "${path}": typeof ${ident},`);
    }
    lines.push(`}>;`);
    for await (const line of codegenApiWithMounts(
      ctx,
      startPush,
      definitionPath,
    )) {
      lines.push(line);
    }
    lines.push(`
      export declare const api: FilterApi<typeof fullApiWithMounts, FunctionReference<any, "public">>;
      export declare const internal: FilterApi<typeof fullApiWithMounts, FunctionReference<any, "internal">>;
    `);
  } else {
    lines.push(header("Generated `api` utility."));
    for (const modulePath of modulePaths) {
      const ident = moduleIdentifier(modulePath);
      const path = importPath(modulePath);
      lines.push(`import type * as ${ident} from "../${path}.js";`);
    }
    lines.push(`
      import type {
        ApiFromModules,
        FunctionReference,
      } from "convex/server";
      /**
       * A utility for referencing Convex functions in your app's API.
       *
       * Usage:
       * \`\`\`js
       * const myFunctionReference = functions.myModule.myFunction;
       * \`\`\`
       */
      declare const functions: ApiFromModules<{
    `);
    for (const modulePath of modulePaths) {
      const ident = moduleIdentifier(modulePath);
      const path = importPath(modulePath);
      lines.push(`  "${path}": typeof ${ident},`);
    }
    lines.push(`}>;`);
  }
  return lines.join("\n");
}

async function* codegenApiWithMounts(
  ctx: Context,
  startPush: StartPushResponse,
  definitionPath: ComponentDefinitionPath,
): AsyncGenerator<string> {
  const mountTree = await buildMountTree(ctx, startPush, definitionPath, []);
  if (!mountTree) {
    yield "declare const fullApiWithMounts: typeof fullApi;";
    return;
  }
  yield "declare const fullApiWithMounts: typeof fullApi &";
  yield* codegenMountTree(mountTree);
  yield `;`;
}

function* codegenMountTree(tree: MountTree): Generator<string> {
  yield `{`;
  for (const [identifier, subtree] of Object.entries(tree)) {
    if (typeof subtree === "string") {
      yield `"${identifier}": ${subtree},`;
    } else {
      yield `"${identifier}":`;
      yield* codegenMountTree(subtree);
      yield `,`;
    }
  }
  yield `}`;
}

interface MountTree {
  [identifier: string]: MountTree | string;
}

async function buildMountTree(
  ctx: Context,
  startPush: StartPushResponse,
  definitionPath: ComponentDefinitionPath,
  attributes: string[],
): Promise<MountTree | null> {
  const analysis = startPush.analysis[definitionPath];
  if (!analysis) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `No analysis found for component ${definitionPath}`,
    });
  }
  let current = analysis.definition.exports.branch;
  for (const attribute of attributes) {
    const componentExport = current.find(
      ([identifier]) => identifier === attribute,
    );
    if (!componentExport) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `No export found for ${attribute}`,
      });
    }
    const [_, node] = componentExport;
    if (node.type !== "branch") {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Expected branch at ${attribute}`,
      });
    }
    current = node.branch;
  }
  return buildComponentMountTree(ctx, startPush, analysis, current);
}

async function buildComponentMountTree(
  ctx: Context,
  startPush: StartPushResponse,
  analysis: EvaluatedComponentDefinition,
  exports: Array<[Identifier, ComponentExports]>,
): Promise<MountTree | null> {
  const result: MountTree = {};
  let nonEmpty = false;
  for (const [identifier, componentExport] of exports) {
    if (componentExport.type === "leaf") {
      // If we're at a child component reference, follow it and build its export tree.
      if (componentExport.leaf.startsWith("_reference/childComponent/")) {
        const suffix = componentExport.leaf.slice(
          "_reference/childComponent/".length,
        );
        const [componentName, ...attributes] = suffix.split("/");
        const childComponent = analysis.definition.childComponents.find(
          (c) => c.name === componentName,
        );
        if (!childComponent) {
          return await ctx.crash({
            exitCode: 1,
            errorType: "fatal",
            printedMessage: `No child component found for ${componentName}`,
          });
        }
        const childTree = await buildMountTree(
          ctx,
          startPush,
          childComponent.path,
          attributes,
        );
        if (childTree) {
          result[identifier] = childTree;
          nonEmpty = true;
        }
      }
      // If we're at a function reference outside the root, codegen it as a leaf.
      const isRoot = analysis.definition.definitionType.type === "app";
      if (!isRoot && componentExport.leaf.startsWith("_reference/function/")) {
        const leaf = await resolveFunctionReference(
          ctx,
          analysis,
          componentExport.leaf,
          "public",
        );
        result[identifier] = leaf;
        nonEmpty = true;
      }
    } else {
      const subTree = await buildComponentMountTree(
        ctx,
        startPush,
        analysis,
        componentExport.branch,
      );
      if (subTree) {
        result[identifier] = subTree;
        nonEmpty = true;
      }
    }
  }
  return nonEmpty ? result : null;
}
