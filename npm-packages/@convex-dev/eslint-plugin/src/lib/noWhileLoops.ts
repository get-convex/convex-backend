import { ESLintUtils } from "@typescript-eslint/utils";
import { isEntryPoint } from "../util.js";

const createRule = ESLintUtils.RuleCreator(
  (name) => `https://docs.convex.io/eslint/${name}`,
);

export const noWhileLoops = createRule({
  name: "no-while-loops",
  meta: {
    type: "suggestion",
    docs: {
      description: "this is a custom rule",
    },
    messages: {
      "no loops": "Loops aren't allowed, they're slow. Try not looping.",
    },
    schema: [],
  },
  defaultOptions: [],
  create: (context) => {
    const filename = context.filename;
    const isGenerated = filename.includes("_generated");

    const entry = isEntryPoint(filename);

    if (!entry || isGenerated) {
      return {};
    }

    return {
      WhileStatement(node) {
        context.report({
          messageId: "no loops",
          node: node,
        });
      },
    };
  },
});
