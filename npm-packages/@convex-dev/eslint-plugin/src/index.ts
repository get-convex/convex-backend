import { noImportUseNode } from "./lib/no-import-use-node.js";
import { noOldRegisteredFunctionSyntax } from "./lib/no-old-registered-function-syntax.js";
import { requireArgsValidator } from "./lib/require-args-validator.js";
import { RuleModule } from "@typescript-eslint/utils/ts-eslint";
import { version } from "./version.js";

const rules = {
  "no-old-registered-function-syntax": noOldRegisteredFunctionSyntax,
  "require-args-validator": requireArgsValidator,
  "import-wrong-runtime": noImportUseNode,
} satisfies Record<string, RuleModule<string, unknown[]>>;

const recommendedRules = {
  // This rule is a good idea but bothersome to convert projects to later:
  // it's possible to safely import specific exports from a "use node"
  // file if all Node.js-specific imports are side-effect free.
  "@convex-dev/import-wrong-runtime": "off",
  "@convex-dev/no-old-registered-function-syntax": "error",
  "@convex-dev/require-args-validator": "error",
} satisfies {
  [key: `@convex-dev/${string}`]: "error" | "warn" | "off";
};

// Bun is hard to feature detect for ESM vs CJS, so only support ESLint 9 with Bun (contributions welcome)
// @ts-expect-error Bun types are not installed
const isBun = typeof Bun !== "undefined";
// Detect ESM to guess at which ESLint version we're using.
const isESM = typeof require === "undefined" || isBun;

// Base plugin structure, common across ESLint 8 and 9
const plugin = {
  // loose types so this can work with ESlint 8 and 9
  configs: {} as {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    recommended: any;
  },
  meta: {
    name: "@convex-dev/eslint-plugin",
    version,
  },
  rules,
  processors: {},
};

// ESLint 9 format (ESM)
if (isESM) {
  Object.assign(plugin.configs, {
    recommended: [
      {
        files: ["**/convex/**/*.ts"],
        plugins: {
          // We could call it "convex" instead, but in ESLint 8 rules can't be renamed like this.
          // For consistency use @convex-dev/rule-name in ESLint 8 and 9.
          "@convex-dev": plugin,
        },
        rules: recommendedRules,
      },
    ],
  });
}
// ESLint 8 format (CommonJS)
else {
  plugin.configs = {
    recommended: {
      // Naming for plugins in namespaced packages is special: it removes the "eslint-plugin" part
      // "plugins": [
      //   "jquery", // means eslint-plugin-jquery
      //   "@jquery/jquery", // means @jquery/eslint-plugin-jquery
      //   "@foobar" // means @foobar/eslint-plugin
      // ]
      // Naming for configs in namespaced packages is also special, but this isn't a config.
      plugins: ["@convex-dev"],
      // Apply no rules globally
      rules: {},
      overrides: [
        {
          // Apply recommended rules in the convex directory
          files: ["**/convex/**/*.ts"],
          rules: recommendedRules,
        },
      ],
    },
  };

  // In CommonJS, we need to directly assign to module.exports
  module.exports = plugin;
}

// For ESM (ESLint 9)
export default plugin;
