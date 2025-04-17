import type { TSESTree } from "@typescript-eslint/utils";
import { CONVEX_REGISTRARS, createRule } from "../util.js";

/**
 * Rule to enforce that every registered Convex function has an args property
 */
export const noMissingArgs = createRule({
  name: "no-missing-args-validator",
  meta: {
    type: "suggestion",
    docs: {
      description: "Every Convex function should validate its arguments.",
    },
    messages: {
      "missing-empty-args": "Convex function is missing args validator.",
      "missing-args":
        "Convex function is missing args validator but has parameter. Add appropriate args validator.",
    },
    schema: [],
    fixable: "code", // Now fixable when handler doesn't have second parameter
    hasSuggestions: true,
  },
  defaultOptions: [],
  create: (context) => {
    // Skip generated files
    const filename = context.getFilename();
    const isGenerated = filename.includes("_generated");
    if (isGenerated) {
      return {};
    }

    /**
     * Checks if an object expression has an args property
     */
    function hasArgsProperty(objectExpr: TSESTree.ObjectExpression): boolean {
      return objectExpr.properties.some(
        (prop) =>
          prop.type === "Property" &&
          prop.key.type === "Identifier" &&
          prop.key.name === "args",
      );
    }

    /**
     * Checks if a handler function has a second parameter (args parameter)
     */
    function handlerHasArgsParameter(handler: TSESTree.Property): boolean {
      if (
        handler.value.type === "ArrowFunctionExpression" ||
        handler.value.type === "FunctionExpression"
      ) {
        return handler.value.params.length >= 2;
      }
      return false;
    }

    /**
     * Gets the handler property from an object expression
     */
    function getHandlerProperty(
      objectExpr: TSESTree.ObjectExpression,
    ): TSESTree.Property | undefined {
      return objectExpr.properties.find(
        (prop) =>
          prop.type === "Property" &&
          prop.key.type === "Identifier" &&
          prop.key.name === "handler",
      ) as TSESTree.Property | undefined;
    }

    return {
      // Check variable declarations for exports that use object syntax
      VariableDeclarator(node) {
        // Only interested in export declarations
        const parentDecl = node.parent;
        if (!parentDecl) return;

        const exportDecl = parentDecl.parent;
        if (
          exportDecl?.type !== "ExportNamedDeclaration" &&
          parentDecl.parent?.parent?.type !== "ExportNamedDeclaration"
        ) {
          return;
        }

        // Check if it's a call to a registrar with an object argument
        if (
          node.init?.type === "CallExpression" &&
          node.init.callee.type === "Identifier" &&
          CONVEX_REGISTRARS.includes(node.init.callee.name) &&
          node.init.arguments.length === 1 &&
          node.init.arguments[0].type === "ObjectExpression"
        ) {
          const objectArg = node.init.arguments[0] as TSESTree.ObjectExpression;

          // Check if the object has an args property
          if (!hasArgsProperty(objectArg)) {
            const handlerProp = getHandlerProperty(objectArg);

            // Determine if the handler has a second parameter
            const handlerHasArgs =
              handlerProp && handlerHasArgsParameter(handlerProp);

            context.report({
              node: objectArg,
              messageId: handlerHasArgs ? "missing-args" : "missing-empty-args",
              // Only provide a fix if the handler doesn't have a second parameter
              fix: handlerHasArgs
                ? undefined
                : (fixer) => {
                    // Find the position to insert args property
                    const sourceCode = context.getSourceCode();
                    const objectText = sourceCode.getText(objectArg);
                    const firstBracePos = objectText.indexOf("{");

                    // Make sure we found the opening brace
                    if (firstBracePos === -1) return null;

                    // Insert args property after the opening brace
                    const insertPos = objectArg.range[0] + firstBracePos + 1;

                    // Add args: {} at the beginning of the object
                    return fixer.insertTextAfterRange(
                      [insertPos, insertPos],
                      "\n  args: {},\n",
                    );
                  },
            });
          }
        }
      },
    };
  },
});

export default noMissingArgs;
