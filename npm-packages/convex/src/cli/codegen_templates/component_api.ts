import path from "path";
import { z } from "zod";
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
import { ComponentDefinitionPath } from "../lib/deployApi/paths.js";
import { Identifier, Reference } from "../lib/deployApi/types.js";
import {
  ConvexValidator,
  convexValidator,
} from "../lib/deployApi/validator.js";
import { CanonicalizedModulePath } from "../lib/deployApi/paths.js";
import { Value, jsonToConvex } from "../../values/value.js";

export function componentApiJs() {
  const lines = [];
  lines.push(header("Generated `api` utility."));
  lines.push(`
    import { anyApi, componentsGeneric } from "convex/server";

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
    export const components = componentsGeneric();
  `);
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

export function componentApiStubDTS() {
  const lines = [];
  lines.push(header("Generated `api` utility."));
  lines.push(`import type { AnyApi, AnyComponents } from "convex/server";`);
  lines.push(`
    export declare const api: AnyApi;
    export declare const internal: AnyApi;
    export declare const components: AnyComponents;
  `);

  return lines.join("\n");
}

export async function componentApiDTS(
  ctx: Context,
  startPush: StartPushResponse,
  rootComponent: ComponentDirectory,
  componentDirectory: ComponentDirectory,
) {
  const definitionPath = toComponentDefinitionPath(
    rootComponent,
    componentDirectory,
  );
  const absModulePaths = await entryPoints(ctx, componentDirectory.path);
  const modulePaths = absModulePaths.map((p) =>
    path.relative(componentDirectory.path, p),
  );

  const lines = [];
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

  lines.push(`
  export declare const components: {`);

  const analysis = startPush.analysis[definitionPath];
  if (!analysis) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `No analysis found for component ${definitionPath} orig: ${definitionPath}\nin\n${Object.keys(startPush.analysis).toString()}`,
    });
  }
  for (const childComponent of analysis.definition.childComponents) {
    const childComponentAnalysis = startPush.analysis[childComponent.path];
    if (!childComponentAnalysis) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `No analysis found for child component ${childComponent.path}`,
      });
    }
    for await (const line of codegenExports(
      ctx,
      childComponent.name,
      childComponentAnalysis,
    )) {
      lines.push(line);
    }
  }

  lines.push("};");

  return lines.join("\n");
}

async function* codegenApiWithMounts(
  ctx: Context,
  startPush: StartPushResponse,
  definitionPath: ComponentDefinitionPath,
): AsyncGenerator<string> {
  const mountTree = await buildMountTree(ctx, startPush, definitionPath, []);
  if (mountTree) {
    yield "export type Mounts = ";
    yield* codegenMountTree(mountTree);
    yield `;`;
    yield `// For now fullApiWithMounts is only fullApi which provides`;
    yield `// jump-to-definition in component client code.`;
    yield `// Use Mounts for the same type without the inference.`;
    yield "declare const fullApiWithMounts: typeof fullApi;";
  } else {
    yield "declare const fullApiWithMounts: typeof fullApi;";
  }
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
  // TODO make these types more precise when receiving analysis from server
  const analysis = startPush.analysis[definitionPath];
  if (!analysis) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `No analysis found for component ${definitionPath} orig: ${definitionPath}\nin\n${Object.keys(startPush.analysis).toString()}`,
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

async function* codegenExports(
  ctx: Context,
  name: Identifier,
  analysis: EvaluatedComponentDefinition,
): AsyncGenerator<string> {
  yield `${name}: {`;
  for (const [name, componentExport] of analysis.definition.exports.branch) {
    yield `${name}:`;
    yield* codegenExport(ctx, analysis, componentExport);
    yield ",";
  }
  yield "},";
}

async function* codegenExport(
  ctx: Context,
  analysis: EvaluatedComponentDefinition,
  componentExport: ComponentExports,
): AsyncGenerator<string> {
  if (componentExport.type === "leaf") {
    yield await resolveFunctionReference(
      ctx,
      analysis,
      componentExport.leaf,
      "internal",
    );
  } else if (componentExport.type === "branch") {
    yield "{";
    for (const [name, childExport] of componentExport.branch) {
      yield `${name}:`;
      yield* codegenExport(ctx, analysis, childExport);
      yield ",";
    }
    yield "}";
  }
}

export async function resolveFunctionReference(
  ctx: Context,
  analysis: EvaluatedComponentDefinition,
  reference: Reference,
  visibility: "public" | "internal",
) {
  if (!reference.startsWith("_reference/function/")) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Invalid function reference: ${reference}`,
    });
  }
  const udfPath = reference.slice("_reference/function/".length);

  const [modulePath, functionName] = udfPath.split(":");
  const canonicalizedModulePath = canonicalizeModulePath(modulePath);

  const analyzedModule = analysis.functions[canonicalizedModulePath];
  if (!analyzedModule) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Module not found: ${modulePath}`,
    });
  }
  const analyzedFunction = analyzedModule.functions.find(
    (f) => f.name === functionName,
  );
  if (!analyzedFunction) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Function not found: ${functionName}`,
    });
  }

  // The server sends down `udfType` capitalized.
  const udfType = analyzedFunction.udfType.toLowerCase();

  let argsType = "any";
  try {
    const argsValidator = parseValidator(analyzedFunction.args);
    if (argsValidator) {
      if (argsValidator.type === "object" || argsValidator.type === "any") {
        argsType = validatorToType(argsValidator);
      } else {
        // eslint-disable-next-line no-restricted-syntax
        throw new Error(
          `Unexpected argument validator type: ${argsValidator.type}`,
        );
      }
    }
  } catch (e) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Invalid function args: ${analyzedFunction.args}`,
      errForSentry: e,
    });
  }

  let returnsType = "any";
  try {
    const returnsValidator = parseValidator(analyzedFunction.returns);
    if (returnsValidator) {
      returnsType = validatorToType(returnsValidator);
    }
  } catch (e) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Invalid function returns: ${analyzedFunction.returns}`,
      errForSentry: e,
    });
  }

  return `FunctionReference<"${udfType}", "${visibility}", ${argsType}, ${returnsType}>`;
}

function parseValidator(validator: string | null): ConvexValidator | null {
  if (!validator) {
    return null;
  }
  return z.nullable(convexValidator).parse(JSON.parse(validator));
}

function canonicalizeModulePath(modulePath: string): CanonicalizedModulePath {
  if (!modulePath.endsWith(".js")) {
    return modulePath + ".js";
  }
  return modulePath;
}

function validatorToType(validator: ConvexValidator): string {
  if (validator.type === "null") {
    return "null";
  } else if (validator.type === "number") {
    return "number";
  } else if (validator.type === "bigint") {
    return "bigint";
  } else if (validator.type === "boolean") {
    return "boolean";
  } else if (validator.type === "string") {
    return "string";
  } else if (validator.type === "bytes") {
    return "ArrayBuffer";
  } else if (validator.type === "any") {
    return "any";
  } else if (validator.type === "literal") {
    const convexValue = jsonToConvex(validator.value);
    return convexValueToLiteral(convexValue);
  } else if (validator.type === "id") {
    return "string";
  } else if (validator.type === "array") {
    return `Array<${validatorToType(validator.value)}>`;
  } else if (validator.type === "record") {
    return `Record<${validatorToType(validator.keys)}, ${validatorToType(validator.values.fieldType)}>`;
  } else if (validator.type === "union") {
    return validator.value.map(validatorToType).join(" | ");
  } else if (validator.type === "object") {
    return objectValidatorToType(validator.value);
  } else {
    // eslint-disable-next-line no-restricted-syntax
    throw new Error(`Unsupported validator type`);
  }
}

function objectValidatorToType(
  fields: Record<string, { fieldType: ConvexValidator; optional: boolean }>,
): string {
  const fieldStrings: string[] = [];
  for (const [fieldName, field] of Object.entries(fields)) {
    const fieldType = validatorToType(field.fieldType);
    fieldStrings.push(`${fieldName}${field.optional ? "?" : ""}: ${fieldType}`);
  }
  return `{ ${fieldStrings.join(", ")} }`;
}

function convexValueToLiteral(value: Value): string {
  if (value === null) {
    return "null";
  }
  if (typeof value === "bigint") {
    return `${value}n`;
  }
  if (typeof value === "number") {
    return `${value}`;
  }
  if (typeof value === "boolean") {
    return `${value}`;
  }
  if (typeof value === "string") {
    return `"${value}"`;
  }
  // eslint-disable-next-line no-restricted-syntax
  throw new Error(`Unsupported literal type`);
}
