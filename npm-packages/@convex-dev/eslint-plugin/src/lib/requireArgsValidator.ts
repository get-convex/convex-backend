import type { TSESTree } from "@typescript-eslint/utils";
import { CONVEX_REGISTRARS, createRule } from "../util.js";
import {
  ReportFixFunction,
  RuleContext,
} from "@typescript-eslint/utils/ts-eslint";

/**
 * Helper function to check if an object expression has an args property
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
 * Helper function to check if a handler function has a non-empty second parameter (args parameter)
 */
function handlerHasArgsParameter(
  handler: TSESTree.ArrowFunctionExpression | TSESTree.FunctionExpression,
): boolean {
  if (handler.params.length < 2) {
    return false;
  }

  // Ignore empty objects
  const secondParam = handler.params[1];
  if (
    secondParam.type === "ObjectPattern" &&
    secondParam.properties.length === 0
  ) {
    return false;
  }

  return true;
}

/**
 * Helper function to get the handler property from an object expression
 */
function getHandlerProperty(
  objectExpr: TSESTree.ObjectExpression,
): TSESTree.ArrowFunctionExpression | TSESTree.FunctionExpression | null {
  const maybeHandler = objectExpr.properties.find(
    (prop) =>
      prop.type === "Property" &&
      prop.key.type === "Identifier" &&
      prop.key.name === "handler",
  ) as TSESTree.Property | undefined;
  if (!maybeHandler) return null;

  if (
    maybeHandler.value.type === "ArrowFunctionExpression" ||
    maybeHandler.value.type === "FunctionExpression"
  ) {
    return maybeHandler.value;
  }

  return null;
}

/**
 * Helper function to create a fix for missing args property
 */
function createArgsFix(
  context: RuleContext<string, unknown[]>,
  objectArg: TSESTree.ObjectExpression,
): ReportFixFunction {
  return (fixer) => {
    const sourceCode = context.getSourceCode();
    const objectText = sourceCode.getText(objectArg);
    const firstBracePos = objectText.indexOf("{");

    if (firstBracePos === -1) return null;

    const insertPos = objectArg.range[0] + firstBracePos + 1;
    return fixer.insertTextAfterRange([insertPos, insertPos], "\n  args: {},");
  };
}

type MessageIds = "missing-empty-args" | "missing-args";

type Options = [
  {
    ignoreUnusedArguments: boolean;
  },
];

/**
 * Rule to enforce that every registered Convex function has an args property
 */
export const requireArgsValidator = createRule<Options, MessageIds>({
  name: "require-args-validator",
  meta: {
    type: "suggestion",
    docs: {
      description: "Require argument validators (`args`) in Convex functions.",
    },
    messages: {
      "missing-empty-args": "Convex function is missing args validator.",
      "missing-args":
        "Convex function is missing args validator but has parameter. Add appropriate args validator.",
    },
    schema: [
      {
        type: "object",
        properties: {
          ignoreUnusedArguments: {
            type: "boolean",
            description:
              "If true, don’t require args validator when function doesn’t use args parameter",
          },
        },
        additionalProperties: false,
      },
    ],
    defaultOptions: [{ ignoreUnusedArguments: false }],
    fixable: "code",
  },
  defaultOptions: [{ ignoreUnusedArguments: false }],
  create: (context, options) => {
    const { ignoreUnusedArguments } = options[0];

    const filename = context.getFilename();
    const isGenerated = filename.includes("_generated");
    if (isGenerated) {
      return {};
    }

    return {
      VariableDeclarator(node) {
        const parentDecl = node.parent;
        if (!parentDecl) return;

        // In an export?
        const exportDecl = parentDecl.parent;
        if (
          exportDecl?.type !== "ExportNamedDeclaration" &&
          parentDecl.parent?.parent?.type !== "ExportNamedDeclaration"
        ) {
          return;
        }

        // Convex function declaration?
        if (
          !(
            node.init?.type === "CallExpression" &&
            node.init.callee.type === "Identifier" &&
            CONVEX_REGISTRARS.includes(node.init.callee.name) &&
            node.init.arguments.length === 1
          )
        )
          return;

        // Old function argument syntax?
        if (
          node.init.arguments[0].type === "ArrowFunctionExpression" ||
          node.init.arguments[0].type === "FunctionExpression"
        ) {
          const handler = node.init.arguments[0];
          if (handlerHasArgsParameter(handler)) {
            context.report({
              node: node.init,
              messageId: "missing-args",
              // Not fixable since we don’t know the type
            });
            return;
          }

          if (!ignoreUnusedArguments) {
            context.report({
              node: node.init,
              messageId: "missing-empty-args",
              fix: (fixer) => {
                let fixText = "{\n";
                fixText += "  args: {},\n";

                // Preserve the original function as much as possible
                const sourceCode = context.getSourceCode();

                // Get the original function text without the outer parentheses
                const originalFunctionText = sourceCode.getText(handler);

                // Add the handler property with the original function
                fixText += `  handler: ${originalFunctionText}`;

                fixText += "\n}";

                return fixer.replaceText(handler, fixText);
              },
            });
          }

          return;
        }

        // New syntax with object argument
        if (node.init.arguments[0].type === "ObjectExpression") {
          const objectArg = node.init.arguments[0] as TSESTree.ObjectExpression;
          if (hasArgsProperty(objectArg)) {
            return;
          }

          const handlerProp = getHandlerProperty(objectArg);
          const handlerHasArgs =
            handlerProp && handlerHasArgsParameter(handlerProp);

          if (!handlerHasArgs && ignoreUnusedArguments) return;

          context.report({
            node: objectArg,
            messageId: handlerHasArgs ? "missing-args" : "missing-empty-args",
            fix: handlerHasArgs ? undefined : createArgsFix(context, objectArg),
          });
        }
      },
    };
  },
});
