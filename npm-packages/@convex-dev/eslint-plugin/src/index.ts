import { noImportUseNode } from "./lib/noImportUseNode.js";
import { noWhileLoops } from "./lib/noWhileLoops.js";

const plugin = {
  meta: {
    name: "@convex-dev/eslint-plugin",
    version: "0.0.0-alpha.0",
  },
  configs: {},
  rules: {
    "no-while-loops": noWhileLoops,
    "import-wrong-runtime": noImportUseNode,
  },
  processors: {},
};

Object.assign(plugin.configs, {
  recommended: [
    {
      files: ["**/convex/**/*.ts"],
      plugins: {
        convex: plugin,
      },
      rules: {
        "convex/import-wrong-runtime": "error",
      },
    },
    {
      files: ["**/convex.config.ts"],
      plugins: {
        convex: plugin,
      },
      rules: {
        // This is an example lint but it would indeed be weird for
        // a component definition to contain a `while(){}` loop.
        "convex/no-while-loops": "error",
      },
    },
  ],
});

// for ESM
export default plugin;
