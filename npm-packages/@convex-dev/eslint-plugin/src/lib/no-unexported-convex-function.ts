import type { TSESTree } from "@typescript-eslint/types";
import { AST_NODE_TYPES } from "@typescript-eslint/types";
import { CONVEX_REGISTRARS, createRule, isEntryPoint } from "../util.js";

type Options = [];
type MessageIds = "no-unexported-convex-function";

/**
 * Rule to disallow declaring Convex functions that are never exported.
 *
 * Convex registers functions by walking the exports of each entry-point
 * module under `convex/`. A top-level `const foo = query({ ... })` that is
 * never exported is dead code at runtime — the registration walker never
 * sees it. This is almost always an oversight (a missing `export` keyword)
 * rather than intentional, and silently fails: the developer thinks they
 * have a callable function but the client cannot reach it.
 *
 * See https://github.com/get-convex/convex-backend/issues/129.
 */
export const noUnexportedConvexFunction = createRule<Options, MessageIds>({
  name: "no-unexported-convex-function",
  meta: {
    type: "problem",
    docs: {
      description:
        "Disallow declaring Convex functions (`query`, `mutation`, `action`, and their `internal*` variants) that are never exported.",
    },
    schema: [],
    messages: {
      "no-unexported-convex-function":
        "Convex function `{{name}}` is declared with `{{registrar}}()` but is never exported. Convex only registers functions that are exported from a module in `convex/`, so this declaration has no effect at runtime — add `export` or remove it.",
    },
  },
  defaultOptions: [],
  create(context) {
    // Only check files that the Convex bundler treats as entry points; helper
    // modules and `_generated/*` are not walked for function registration.
    if (!isEntryPoint(context.filename)) return {};

    const candidates = new Map<
      string,
      { node: TSESTree.Node; registrar: string }
    >();
    const exportedNames = new Set<string>();

    return {
      VariableDeclaration(node) {
        const parent = node.parent;
        if (!parent) return;
        const isExported =
          parent.type === AST_NODE_TYPES.ExportNamedDeclaration;
        const atTopLevel =
          parent.type === AST_NODE_TYPES.Program ||
          (isExported && parent.parent?.type === AST_NODE_TYPES.Program);
        if (!atTopLevel) return;

        for (const declarator of node.declarations) {
          if (declarator.id.type !== AST_NODE_TYPES.Identifier) continue;
          const init = declarator.init;
          if (!init || init.type !== AST_NODE_TYPES.CallExpression) continue;
          if (init.callee.type !== AST_NODE_TYPES.Identifier) continue;
          if (!CONVEX_REGISTRARS.includes(init.callee.name)) continue;

          if (isExported) {
            exportedNames.add(declarator.id.name);
          } else {
            candidates.set(declarator.id.name, {
              node: declarator,
              registrar: init.callee.name,
            });
          }
        }
      },
      // `export { foo }` and `export { foo as bar }` (re-exports too).
      ExportSpecifier(node) {
        if (node.local.type === AST_NODE_TYPES.Identifier) {
          exportedNames.add(node.local.name);
        }
      },
      // `export default foo` where `foo` is a previously-declared identifier.
      ExportDefaultDeclaration(node) {
        if (node.declaration.type === AST_NODE_TYPES.Identifier) {
          exportedNames.add(node.declaration.name);
        }
      },
      "Program:exit"() {
        for (const [name, { node, registrar }] of candidates) {
          if (exportedNames.has(name)) continue;
          context.report({
            node,
            messageId: "no-unexported-convex-function",
            data: { name, registrar },
          });
        }
      },
    };
  },
});
