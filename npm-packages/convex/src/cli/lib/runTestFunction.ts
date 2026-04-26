import type { Context } from "../../bundler/context.js";
import { deploymentFetch } from "./utils/utils.js";

export type TestFunctionUdfType = "query" | "mutation";

const QUERY_MODULE_PREAMBLE =
  'import { query, internalQuery } from "convex:/_system/repl/wrappers.js";';
const MUTATION_MODULE_PREAMBLE =
  'import { mutation, internalMutation } from "convex:/_system/repl/wrappers.js";';

/** Shared help text for the query/module string (CLI argument + MCP input). */
export const RUN_ONEOFF_QUERY_SOURCE_DESCRIPTION =
  'JavaScript module source for a single file (testQuery.js) that exports a default readonly query, for example: export default query({ handler: async (ctx) => ({ count: (await ctx.db.query("messages").take(10)).length }) });';

/** Shared help text for the mutation/module string (CLI argument). */
export const RUN_ONEOFF_MUTATION_SOURCE_DESCRIPTION =
  'JavaScript module source for a single file (testMutation.js) that exports a default mutation, for example: export default mutation({ handler: async (ctx) => { const id = await ctx.db.insert("messages", { body: "hello" }); return { id }; } });';

export const INLINE_QUERY_DESCRIPTION =
  "JavaScript to evaluate as a readonly query, for example: 'await ctx.db.query(\"messages\").take(5)'. Simple expressions are returned automatically. For multi-statement queries, use an explicit return. Full `export default query(...)` modules are also supported. This is a one-shot query and cannot be combined with `--watch`. Use `--component` to target a mounted component. For more examples, see `npx convex docs`.";

export const INLINE_MUTATION_DESCRIPTION =
  'JavaScript to evaluate as a one-shot mutation, for example: \'await ctx.db.insert("messages", { body: "hello" })\'. Simple expressions are returned automatically. For multi-statement mutations, use an explicit return. Full `export default mutation(...)` modules are also supported. This is a one-shot mutation and cannot be combined with `--watch`. Use `--component` to target a mounted component. For more examples, see `npx convex docs`.';

export type RunTestFunctionQuerySuccess = {
  kind: "success";
  value: unknown;
  logLines: string[];
};

export type RunTestFunctionQueryApplicationFailure = {
  kind: "applicationFailure";
  payload: unknown;
};

export type RunTestFunctionQueryResult =
  | RunTestFunctionQuerySuccess
  | RunTestFunctionQueryApplicationFailure;

export function inlineQueryToQuerySource(inlineQuery: string) {
  return inlineFunctionToSource("query", inlineQuery);
}

export function inlineMutationToMutationSource(inlineMutation: string) {
  return inlineFunctionToSource("mutation", inlineMutation);
}

/**
 * POST /api/run_test_function with the same body shape as the dashboard and MCP.
 * Uses deploymentFetch for Convex-Client, auth headers, retries, and error typing.
 * On HTTP failure, throws ThrowingFetchError (from deploymentFetch).
 */
export async function runTestFunctionQuery(
  ctx: Context,
  args: {
    deploymentUrl: string;
    adminKey: string;
    querySource: string;
    componentId?: string;
  },
): Promise<RunTestFunctionQueryResult> {
  return await runTestFunction(ctx, {
    deploymentUrl: args.deploymentUrl,
    adminKey: args.adminKey,
    udfType: "query",
    functionSource: args.querySource,
    ...(args.componentId !== undefined
      ? { componentId: args.componentId }
      : {}),
  });
}

export async function runTestFunctionMutation(
  ctx: Context,
  args: {
    deploymentUrl: string;
    adminKey: string;
    mutationSource: string;
    componentId?: string;
  },
): Promise<RunTestFunctionQueryResult> {
  return await runTestFunction(ctx, {
    deploymentUrl: args.deploymentUrl,
    adminKey: args.adminKey,
    udfType: "mutation",
    functionSource: args.mutationSource,
    ...(args.componentId !== undefined
      ? { componentId: args.componentId }
      : {}),
  });
}

async function runTestFunction(
  ctx: Context,
  args: {
    deploymentUrl: string;
    adminKey: string;
    udfType: TestFunctionUdfType;
    functionSource: string;
    componentId?: string;
  },
): Promise<RunTestFunctionQueryResult> {
  const fetchDeployment = deploymentFetch(ctx, {
    deploymentUrl: args.deploymentUrl,
    adminKey: args.adminKey,
  });
  const response = await fetchDeployment("/api/run_test_function", {
    method: "POST",
    body: JSON.stringify({
      adminKey: args.adminKey,
      args: {},
      bundle: {
        path:
          args.udfType === "query"
            ? "__convex_repl__/testQuery.js"
            : "__convex_repl__/testMutation.js",
        source: args.functionSource,
      },
      udfType: args.udfType,
      format: "convex_encoded_json",
      ...(args.componentId !== undefined
        ? { componentId: args.componentId }
        : {}),
    }),
  });
  const result: unknown = await response.json();
  if (
    typeof result !== "object" ||
    result === null ||
    !("status" in result) ||
    (result as { status: string }).status !== "success"
  ) {
    return { kind: "applicationFailure", payload: result };
  }
  const ok = result as {
    status: "success";
    value: unknown;
    logLines?: string[];
  };
  return {
    kind: "success",
    value: ok.value,
    logLines: ok.logLines ?? [],
  };
}

function inlineFunctionToSource(
  udfType: TestFunctionUdfType,
  inlineSource: string,
) {
  const trimmedSource = inlineSource.trim();
  if (looksLikeFunctionModuleSource(udfType, trimmedSource)) {
    return injectFunctionModulePreamble(udfType, trimmedSource);
  }

  const functionBody = inlineFunctionBody(trimmedSource);
  const wrapperName = udfType === "query" ? "query" : "mutation";
  return `${modulePreamble(udfType)}

export default ${wrapperName}({
  handler: async (ctx) => {
${indent(functionBody, 4)}
  },
});`;
}

function looksLikeFunctionModuleSource(
  udfType: TestFunctionUdfType,
  source: string,
) {
  if (!source.includes("export default")) return false;
  return udfType === "query"
    ? /\b(?:query|internalQuery)\s*\(/.test(source)
    : /\b(?:mutation|internalMutation)\s*\(/.test(source);
}

function injectFunctionModulePreamble(
  udfType: TestFunctionUdfType,
  source: string,
) {
  if (source.includes("convex:/_system/repl/wrappers.js")) return source;
  return `${modulePreamble(udfType)}

${source}`;
}

function modulePreamble(udfType: TestFunctionUdfType) {
  return udfType === "query" ? QUERY_MODULE_PREAMBLE : MUTATION_MODULE_PREAMBLE;
}

function inlineFunctionBody(inlineSource: string) {
  const trimmed = inlineSource.trim();
  if (!isExpression(trimmed)) return trimmed;
  return `return (${trimmed.replace(/;$/, "")});`;
}

function isExpression(inlineQuery: string) {
  if (inlineQuery.includes("\n")) return false;
  return !/^(const|let|var|if|for|while|switch|try|throw|return)\b/.test(
    inlineQuery,
  );
}

function indent(text: string, spaces: number) {
  const prefix = " ".repeat(spaces);
  return text
    .split("\n")
    .map((line) => `${prefix}${line}`)
    .join("\n");
}
