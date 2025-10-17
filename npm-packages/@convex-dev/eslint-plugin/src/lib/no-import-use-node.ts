import path from "path";
import fs from "fs";
import { createRule, isEntryPoint } from "../util.js";

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
    const filename = context.getFilename();
    const entry = isEntryPoint(context.getFilename());
    if (!entry) return {};

    const currentDir = path.dirname(filename);
    return {
      ImportDeclaration(node) {
        if (typeof node.source.value !== "string") return {};
        const relative = node.source.value;
        if (!relative.startsWith(".")) return {};
        const abs = path.resolve(currentDir, relative);

        // TODO this is a heuristic, find out about convex.json
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
