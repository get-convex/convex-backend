import type { TSESTree } from "@typescript-eslint/utils";
import { CONVEX_REGISTRARS, createRule } from "../util.js";

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
 * Helper function to check if a handler function has a second parameter (args parameter)
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
 * Helper function to get the handler property from an object expression
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

/**
 * Helper function to create a fix for missing args property
 */
function createArgsFix(
  context: any,
  objectArg: TSESTree.ObjectExpression,
): ((fixer: any) => any) | undefined {
  return (fixer) => {
    const sourceCode = context.getSourceCode();
    const objectText = sourceCode.getText(objectArg);
    const firstBracePos = objectText.indexOf("{");

    if (firstBracePos === -1) return null;

    const insertPos = objectArg.range[0] + firstBracePos + 1;
    return fixer.insertTextAfterRange(
      [insertPos, insertPos],
      "\n  args: {},\n",
    );
  };
}

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
    fixable: "code",
  },
  defaultOptions: [],
  create: (context) => {
    const filename = context.getFilename();
    const isGenerated = filename.includes("_generated");
    if (isGenerated) {
      return {};
    }

    return {
      VariableDeclarator(node) {
        const parentDecl = node.parent;
        if (!parentDecl) return;

        const exportDecl = parentDecl.parent;
        if (
          exportDecl?.type !== "ExportNamedDeclaration" &&
          parentDecl.parent?.parent?.type !== "ExportNamedDeclaration"
        ) {
          return;
        }

        if (
          node.init?.type === "CallExpression" &&
          node.init.callee.type === "Identifier" &&
          CONVEX_REGISTRARS.includes(node.init.callee.name) &&
          node.init.arguments.length === 1 &&
          node.init.arguments[0].type === "ObjectExpression"
        ) {
          const objectArg = node.init.arguments[0] as TSESTree.ObjectExpression;

          if (!hasArgsProperty(objectArg)) {
            const handlerProp = getHandlerProperty(objectArg);
            const handlerHasArgs =
              handlerProp && handlerHasArgsParameter(handlerProp);

            context.report({
              node: objectArg,
              messageId: handlerHasArgs ? "missing-args" : "missing-empty-args",
              fix: handlerHasArgs
                ? undefined
                : createArgsFix(context, objectArg),
            });
          }
        }
      },
    };
  },
});

/**
 * Rule to enforce that Convex functions with args parameters have args validators
 */
export const noArgsWithoutValidator = createRule({
  name: "no-args-without-validator",
  meta: {
    type: "suggestion",
    docs: {
      description:
        "Convex functions with args parameters should validate their arguments.",
    },
    messages: {
      "missing-args":
        "Convex function is missing args validator but has parameter. Add appropriate args validator.",
    },
    schema: [],
    fixable: "code",
  },
  defaultOptions: [],
  create: (context) => {
    const filename = context.getFilename();
    const isGenerated = filename.includes("_generated");
    if (isGenerated) {
      return {};
    }

    return {
      VariableDeclarator(node) {
        const parentDecl = node.parent;
        if (!parentDecl) return;

        const exportDecl = parentDecl.parent;
        if (
          exportDecl?.type !== "ExportNamedDeclaration" &&
          parentDecl.parent?.parent?.type !== "ExportNamedDeclaration"
        ) {
          return;
        }

        if (
          node.init?.type === "CallExpression" &&
          node.init.callee.type === "Identifier" &&
          CONVEX_REGISTRARS.includes(node.init.callee.name) &&
          node.init.arguments.length === 1 &&
          node.init.arguments[0].type === "ObjectExpression"
        ) {
          const objectArg = node.init.arguments[0] as TSESTree.ObjectExpression;
          const handlerProp = getHandlerProperty(objectArg);
          const handlerHasArgs =
            handlerProp && handlerHasArgsParameter(handlerProp);

          if (handlerHasArgs && !hasArgsProperty(objectArg)) {
            context.report({
              node: objectArg,
              messageId: "missing-args",
              fix: createArgsFix(context, objectArg),
            });
          }
        }
      },
    };
  },
});
