import path from "path";
import fs from "fs";
import {
  AST_NODE_TYPES,
  ESLintUtils,
  TSESTree,
} from "@typescript-eslint/utils";
import { isEntryPoint } from "../util.js";

const createRule = ESLintUtils.RuleCreator(
  (name) => `https://docs.convex.io/eslint/${name}`,
);

export const noImportUseNode = createRule({
  name: "no-import-use-node",
  meta: {
    type: "suggestion",
    docs: {
      description:
        'Only "use node" modules can import other "use node" modules',
    },
    messages: {
      "wrong runtime import":
        'This file uses the Convex JavaScript runtime but it imports a "use node" module.',
    },
    schema: [],
  },
  defaultOptions: [],
  create: (context) => {
    const filename = context.filename;
    const isGenerated = filename.includes("_generated");

    const entry = isEntryPoint(filename);
    // This is a heuristic: it's possible to name the convex dir anything!
    // TODO check ancestor directories for a convex.json and use that.
    const isInConvexDir = filename.includes("convex" + path.sep);

    if (!entry || isGenerated) return {};

    const currentDir = path.dirname(context.filename);
    let isNodeJs: null | boolean = null;
    return {
      Program(node) {
        isNodeJs = isUseNode(node);
      },
      ImportDeclaration(node) {
        if (typeof node.source.value !== "string") return {};
        const relative = node.source.value;
        if (!relative.startsWith(".")) return {};
        const abs = path.resolve(currentDir, relative);

        // TODO this is a heuristic, findout about convex.json
        if (!abs.includes("convex/")) return {};
        const sourceFile = resolveFile(abs);
        if (!sourceFile) return;
        let source;
        try {
          source = fs.readFileSync(sourceFile, { encoding: "utf-8" });
        } catch {
          return;
        }
        if (source && source.slice(0, 100).includes("use node")) {
          context.report({
            messageId: "wrong runtime import",
            node: node,
          });
        }
      },
    };
  },
});

function isUseNode(node: TSESTree.Program) {
  const first = node.body[0];
  if (!first) return false;
  if (first.type !== AST_NODE_TYPES.ExpressionStatement) return false;
  if (first.expression.type !== AST_NODE_TYPES.Literal) return false;
  return first.expression.value === "use node";
}

// Implement basic module resolution for relative paths only.
// This doesn't work with path aliases and so many other cases;
// it's a proof of concept that might be helpful to folks debugging.
function resolveFile(filePath: string): string | null {
  const extensions = [".ts", ".tsx", ".js", ".jsx", ""];
  for (const ext of extensions) {
    const fullPath = `${filePath}${ext}`;
    if (fs.existsSync(fullPath) && fs.statSync(fullPath).isFile()) {
      return fullPath;
    }
  }

  // Check for directory import
  if (fs.existsSync(filePath) && fs.statSync(filePath).isDirectory()) {
    for (const ext of extensions) {
      const indexPath = path.join(filePath, `index${ext}`);
      if (fs.existsSync(indexPath) && fs.statSync(indexPath).isFile()) {
        return indexPath;
      }
    }
  }

  return null;
}
