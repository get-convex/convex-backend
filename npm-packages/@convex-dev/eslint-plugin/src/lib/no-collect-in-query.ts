import type { TSESTree } from "@typescript-eslint/utils";
import { AST_NODE_TYPES } from "@typescript-eslint/utils";
import { createRule } from "../util.js";
import type ts from "typescript";

type Options = [];
type MessageIds =
  | "no-collect-in-query"
  | "replace-with-paginate"
  | "replace-with-take";

function isCollectCall(
  node: TSESTree.CallExpression,
): node is TSESTree.CallExpression & {
  callee: TSESTree.MemberExpression & {
    computed: false;
    property: TSESTree.Identifier & { name: "collect" };
  };
} {
  if (node.callee.type !== AST_NODE_TYPES.MemberExpression) return false;
  if (node.callee.computed) return false;
  if (node.callee.property.type !== AST_NODE_TYPES.Identifier) return false;
  return node.callee.property.name === "collect";
}

function findOrderedQueryType(
  program: ts.Program,
  checker: ts.TypeChecker,
): ts.Type | null {
  try {
    for (const sf of program.getSourceFiles()) {
      // Prefer the source file from the convex package.
      // In this monorepo it's `npm-packages/convex/src/server/query.ts`, but be
      // permissive about path separators and .d.ts builds.
      const fname = sf.fileName.replace(/\\/g, "/");
      if (!fname.includes("/convex/")) continue;
      if (
        !fname.endsWith("/server/query.ts") &&
        !fname.endsWith("/server/query.d.ts")
      ) {
        continue;
      }
      const sourceFileSymbol = checker.getSymbolAtLocation(sf);
      if (!sourceFileSymbol) continue;
      const exports = checker.getExportsOfModule(sourceFileSymbol);
      const orderedQuerySymbol = exports.find((e) => e.name === "OrderedQuery");
      if (!orderedQuerySymbol) continue;
      return checker.getTypeOfSymbolAtLocation(orderedQuerySymbol, sf);
    }
  } catch {
    // ignore and fall back
  }
  return null;
}

/**
 * Rule to discourage calling `.collect()` on Convex queries.
 */
export const noCollectInQuery = createRule<Options, MessageIds>({
  name: "no-collect-in-query",
  meta: {
    type: "suggestion",
    docs: {
      description:
        "Disallow calling `.collect()` on Convex queries; prefer `.take()` or `.paginate()` instead.",
    },
    schema: [],
    hasSuggestions: true,
    messages: {
      "no-collect-in-query":
        "Avoid calling `.collect()` in a Convex query: it can fail for large datasets. Prefer `.take()` or `.paginate()` instead (see the best practices docs). If you are certain that this call to `.collect()` won’t reach the [Convex query limits](https://docs.convex.dev/production/state/limits), you can disable this line with `// eslint-disable-next-line @convex-dev/no-collect-in-query`.",
      "replace-with-take": "Replace `.collect()` with `.take()`.",
      "replace-with-paginate": "Replace `.collect()` with `.paginate()`.",
    },
  },
  defaultOptions: [],
  create: (context) => {
    const filename = context.filename;

    // Generated files don’t use the DB APIs, so we skip them to avoid unnecessary work
    if (filename.includes("_generated")) {
      return {};
    }

    const services = context.sourceCode.parserServices;
    if (
      !services?.program ||
      !services.esTreeNodeToTSNodeMap ||
      typeof services.esTreeNodeToTSNodeMap.get !== "function"
    ) {
      // Type information not available.
      return {};
    }

    const checker = services.program.getTypeChecker();
    const tsNodeMap = services.esTreeNodeToTSNodeMap;

    // Resolve the `OrderedQuery` type from the convex package once.
    const orderedQueryType = findOrderedQueryType(services.program, checker);
    if (!orderedQueryType) {
      // If we can't find the `OrderedQuery` type, skip to avoid false positives.
      return {};
    }

    return {
      CallExpression(node) {
        if (!isCollectCall(node)) return;

        // Avoid warning on `any` or other unresolved types.
        const objectTsNode = tsNodeMap.get(node.callee.object);
        const objectType = checker.getTypeAtLocation(objectTsNode);
        if (checker.typeToString(objectType) === "any") {
          return;
        }

        if (!checker.isTypeAssignableTo(objectType, orderedQueryType)) {
          return;
        }

        context.report({
          node,
          messageId: "no-collect-in-query",
          suggest: [
            {
              messageId: "replace-with-take",
              fix: (fixer) => fixer.replaceText(node.callee.property, "take"),
            },
            {
              messageId: "replace-with-paginate",
              fix: (fixer) =>
                fixer.replaceText(node.callee.property, "paginate"),
            },
          ],
        });
      },
    };
  },
});
