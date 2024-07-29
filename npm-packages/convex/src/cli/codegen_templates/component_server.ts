import {
  ComponentDirectory,
  toComponentDefinitionPath,
} from "../lib/components/definition/directoryStructure.js";
import {
  ComponentExports,
  EvaluatedComponentDefinition,
} from "../lib/deployApi/componentDefinition.js";
import { Identifier, Reference } from "../lib/deployApi/types.js";
import { StartPushResponse } from "../lib/deployApi/startPush.js";
import {
  ConvexValidator,
  convexValidator,
} from "../lib/deployApi/validator.js";
import { header } from "./common.js";
import { Context, logError } from "../../bundler/context.js";
import { CanonicalizedModulePath } from "../lib/deployApi/paths.js";
import { Value, jsonToConvex } from "../../values/value.js";
import { z } from "zod";

export function componentServerJS(isRoot: boolean): string {
  let result = `
  ${header(
    "Generated utilities for implementing server-side Convex query and mutation functions.",
  )}
  import {
    actionGeneric,
    httpActionGeneric,
    queryGeneric,
    mutationGeneric,
    internalActionGeneric,
    internalMutationGeneric,
    internalQueryGeneric,
    appGeneric,
    componentGeneric,
    createComponentArgs,
  } from "convex/server";

  /**
   * Define a query in this Convex app's public API.
   *
   * This function will be allowed to read your Convex database and will be accessible from the client.
   *
   * @param func - The query function. It receives a {@link QueryCtx} as its first argument.
   * @returns The wrapped query. Include this as an \`export\` to name it and make it accessible.
   */
  export const query = queryGeneric;

  /**
   * Define a query that is only accessible from other Convex functions (but not from the client).
   *
   * This function will be allowed to read from your Convex database. It will not be accessible from the client.
   *
   * @param func - The query function. It receives a {@link QueryCtx} as its first argument.
   * @returns The wrapped query. Include this as an \`export\` to name it and make it accessible.
   */
  export const internalQuery = internalQueryGeneric;

  /**
   * Define a mutation in this Convex app's public API.
   *
   * This function will be allowed to modify your Convex database and will be accessible from the client.
   *
   * @param func - The mutation function. It receives a {@link MutationCtx} as its first argument.
   * @returns The wrapped mutation. Include this as an \`export\` to name it and make it accessible.
   */
  export const mutation = mutationGeneric;

  /**
   * Define a mutation that is only accessible from other Convex functions (but not from the client).
   *
   * This function will be allowed to modify your Convex database. It will not be accessible from the client.
   *
   * @param func - The mutation function. It receives a {@link MutationCtx} as its first argument.
   * @returns The wrapped mutation. Include this as an \`export\` to name it and make it accessible.
   */
  export const internalMutation = internalMutationGeneric;

  /**
   * Define an action in this Convex app's public API.
   *
   * An action is a function which can execute any JavaScript code, including non-deterministic
   * code and code with side-effects, like calling third-party services.
   * They can be run in Convex's JavaScript environment or in Node.js using the "use node" directive.
   * They can interact with the database indirectly by calling queries and mutations using the {@link ActionCtx}.
   *
   * @param func - The action. It receives an {@link ActionCtx} as its first argument.
   * @returns The wrapped action. Include this as an \`export\` to name it and make it accessible.
   */
  export const action = actionGeneric;

  /**
   * Define an action that is only accessible from other Convex functions (but not from the client).
   *
   * @param func - The function. It receives an {@link ActionCtx} as its first argument.
   * @returns The wrapped function. Include this as an \`export\` to name it and make it accessible.
   */
  export const internalAction = internalActionGeneric;

  /**
   * Define a Convex HTTP action.
   *
   * @param func - The function. It receives an {@link ActionCtx} as its first argument, and a \`Request\` object
   * as its second.
   * @returns The wrapped endpoint function. Route a URL path to this function in \`convex/http.js\`.
   */
  export const httpAction = httpActionGeneric;
  `;
  if (isRoot) {
    result += `
    export const app = appGeneric();
    `;
  } else {
    result += `
    export const component = componentGeneric();
    export const componentArgs = createComponentArgs();
    `;
  }
  return result;
}

function componentServerDTSPrelude(): string {
  return `
    ${header(
      "Generated utilities for implementing server-side Convex query and mutation functions.",
    )}
    import {
      ActionBuilder,
      HttpActionBuilder,
      MutationBuilderWithTable,
      QueryBuilderWithTable,
      GenericActionCtx,
      GenericMutationCtxWithTable,
      GenericQueryCtxWithTable,
      GenericDatabaseReaderWithTable,
      GenericDatabaseWriterWithTable,
      FunctionReference,
    } from "convex/server";
    import type { DataModel } from "./dataModel.js";

    /**
     * Define a query in this Convex app's public API.
     *
     * This function will be allowed to read your Convex database and will be accessible from the client.
     *
     * @param func - The query function. It receives a {@link QueryCtx} as its first argument.
     * @returns The wrapped query. Include this as an \`export\` to name it and make it accessible.
     */
    export declare const query: QueryBuilderWithTable<DataModel, "public">;

    /**
     * Define a query that is only accessible from other Convex functions (but not from the client).
     *
     * This function will be allowed to read from your Convex database. It will not be accessible from the client.
     *
     * @param func - The query function. It receives a {@link QueryCtx} as its first argument.
     * @returns The wrapped query. Include this as an \`export\` to name it and make it accessible.
     */
    export declare const internalQuery: QueryBuilderWithTable<DataModel, "internal">;

    /**
     * Define a mutation in this Convex app's public API.
     *
     * This function will be allowed to modify your Convex database and will be accessible from the client.
     *
     * @param func - The mutation function. It receives a {@link MutationCtx} as its first argument.
     * @returns The wrapped mutation. Include this as an \`export\` to name it and make it accessible.
     */
    export declare const mutation: MutationBuilderWithTable<DataModel, "public">;

    /**
     * Define a mutation that is only accessible from other Convex functions (but not from the client).
     *
     * This function will be allowed to modify your Convex database. It will not be accessible from the client.
     *
     * @param func - The mutation function. It receives a {@link MutationCtx} as its first argument.
     * @returns The wrapped mutation. Include this as an \`export\` to name it and make it accessible.
     */
    export declare const internalMutation: MutationBuilderWithTable<DataModel, "internal">;

    /**
     * Define an action in this Convex app's public API.
     *
     * An action is a function which can execute any JavaScript code, including non-deterministic
     * code and code with side-effects, like calling third-party services.
     * They can be run in Convex's JavaScript environment or in Node.js using the "use node" directive.
     * They can interact with the database indirectly by calling queries and mutations using the {@link ActionCtx}.
     *
     * @param func - The action. It receives an {@link ActionCtx} as its first argument.
     * @returns The wrapped action. Include this as an \`export\` to name it and make it accessible.
     */
    export declare const action: ActionBuilder<DataModel, "public">;

    /**
     * Define an action that is only accessible from other Convex functions (but not from the client).
     *
     * @param func - The function. It receives an {@link ActionCtx} as its first argument.
     * @returns The wrapped function. Include this as an \`export\` to name it and make it accessible.
     */
    export declare const internalAction: ActionBuilder<DataModel, "internal">;

    /**
     * Define an HTTP action.
     *
     * This function will be used to respond to HTTP requests received by a Convex
     * deployment if the requests matches the path and method where this action
     * is routed. Be sure to route your action in \`convex/http.js\`.
     *
     * @param func - The function. It receives an {@link ActionCtx} as its first argument.
     * @returns The wrapped function. Import this function from \`convex/http.js\` and route it to hook it up.
     */
    export declare const httpAction: HttpActionBuilder;

    /**
     * A set of services for use within Convex query functions.
     *
     * The query context is passed as the first argument to any Convex query
     * function run on the server.
     *
     * This differs from the {@link MutationCtx} because all of the services are
     * read-only.
     */
    export type QueryCtx = GenericQueryCtxWithTable<DataModel>;

    /**
     * A set of services for use within Convex mutation functions.
     *
     * The mutation context is passed as the first argument to any Convex mutation
     * function run on the server.
     */
    export type MutationCtx = GenericMutationCtxWithTable<DataModel>;

    /**
     * A set of services for use within Convex action functions.
     *
     * The action context is passed as the first argument to any Convex action
     * function run on the server.
     */
    export type ActionCtx = GenericActionCtx<DataModel>;

    /**
     * An interface to read from the database within Convex query functions.
     *
     * The two entry points are {@link DatabaseReader.get}, which fetches a single
     * document by its {@link Id}, or {@link DatabaseReader.query}, which starts
     * building a query.
     */
    export type DatabaseReader = GenericDatabaseReaderWithTable<DataModel>;

    /**
     * An interface to read from and write to the database within Convex mutation
     * functions.
     *
     * Convex guarantees that all writes within a single mutation are
     * executed atomically, so you never have to worry about partial writes leaving
     * your data in an inconsistent state. See [the Convex Guide](https://docs.convex.dev/understanding/convex-fundamentals/functions#atomicity-and-optimistic-concurrency-control)
     * for the guarantees Convex provides your functions.
     */
    export type DatabaseWriter = GenericDatabaseWriterWithTable<DataModel>;
  `;
}

export function componentServerStubDTS(isRoot: boolean): string {
  let result = componentServerDTSPrelude();
  if (isRoot) {
    result += `
    export declare const app: AnyApp;
    `;
  } else {
    result += `
    export declare const component: AnyComponent;
    export declare const componentArgs: Record<string, any>;
    `;
  }
  return result;
}

export async function componentServerDTS(
  ctx: Context,
  startPush: StartPushResponse,
  rootComponent: ComponentDirectory,
  componentDirectory: ComponentDirectory,
): Promise<string> {
  const result = [componentServerDTSPrelude()];

  const identifier = componentDirectory.isRoot ? "app" : "component";
  result.push(`export declare const ${identifier}: {`);

  const definitionPath = toComponentDefinitionPath(
    rootComponent,
    componentDirectory,
  );

  const analysis = startPush.analysis[definitionPath];
  if (!analysis) {
    logError(ctx, `No analysis found for component ${definitionPath}`);
    return await ctx.crash(1, "fatal");
  }
  for (const childComponent of analysis.definition.childComponents) {
    const childComponentAnalysis = startPush.analysis[childComponent.path];
    if (!childComponentAnalysis) {
      logError(
        ctx,
        `No analysis found for child component ${childComponent.path}`,
      );
      return await ctx.crash(1, "fatal");
    }
    for await (const line of codegenExports(
      ctx,
      childComponent.name,
      childComponentAnalysis,
    )) {
      result.push(line);
    }
    result.push(`${childComponent.name}: {},`);
  }

  result.push("};");

  const definitionType = analysis.definition.definitionType;
  if (definitionType.type === "childComponent") {
    result.push(`export declare const componentArgs: {`);
    for (const [name, { value: serializedValidator }] of definitionType.args) {
      const validatorType = validatorToType(JSON.parse(serializedValidator));
      result.push(`${name}: ${validatorType},`);
    }
    result.push("};");
  }

  return result.join("\n");
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
    yield await resolveFunctionReference(ctx, analysis, componentExport.leaf);
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

async function resolveFunctionReference(
  ctx: Context,
  analysis: EvaluatedComponentDefinition,
  reference: Reference,
) {
  if (!reference.startsWith("_reference/function/")) {
    logError(ctx, `Invalid function reference: ${reference}`);
    return await ctx.crash(1, "fatal");
  }
  const udfPath = reference.slice("_reference/function/".length);

  const [modulePath, functionName] = udfPath.split(":");
  const canonicalizedModulePath = canonicalizeModulePath(modulePath);

  const analyzedModule = analysis.functions[canonicalizedModulePath];
  if (!analyzedModule) {
    logError(ctx, `Module not found: ${modulePath}`);
    return await ctx.crash(1, "fatal");
  }
  const analyzedFunction = analyzedModule.functions.find(
    (f) => f.name === functionName,
  );
  if (!analyzedFunction) {
    logError(ctx, `Function not found: ${functionName}`);
    return await ctx.crash(1, "fatal");
  }

  // The server sends down `udfType` capitalized.
  const udfType = analyzedFunction.udfType.toLowerCase();

  const argsValidator = parseValidator(analyzedFunction.args);
  let argsType = "any";
  if (argsValidator) {
    if (argsValidator.type === "object" || argsValidator.type === "any") {
      argsType = validatorToType(argsValidator);
    } else {
      logError(ctx, `Invalid function args: ${analyzedFunction.args}`);
      return await ctx.crash(1, "fatal");
    }
  }

  const returnsValidator = parseValidator(analyzedFunction.returns);
  let returnsType = "any";
  if (returnsValidator) {
    returnsType = validatorToType(returnsValidator);
  }

  return `FunctionReference<"${udfType}", "internal", ${argsType}, ${returnsType}>`;
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
    return `Record<${validatorToType(validator.keys)}, ${validatorToType(validator.values)}>`;
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
