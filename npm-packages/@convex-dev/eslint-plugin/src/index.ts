import { noImportUseNode } from "./lib/noImportUseNode.js";
import noOldRegisteredFunctionSyntax from "./lib/noOldRegisteredFunctionSyntax.js";
import noMissingArgs from "./lib/noMissingArgs.js";
import { RuleModule } from "@typescript-eslint/utils/ts-eslint";

const rules = {
  "no-old-registered-function-syntax": noOldRegisteredFunctionSyntax,
  "no-missing-args-validator": noMissingArgs,
  "import-wrong-runtime": noImportUseNode,
} satisfies Record<string, RuleModule<any>>;

const recommendedRules = {
  // This rule is a good idea but hard to convert projects to later.
  "@convex-dev/import-wrong-runtime": "off",
  "@convex-dev/no-old-registered-function-syntax": "error",
  "@convex-dev/no-missing-args-validator": "error",
} satisfies {
  [key: `@convex-dev/${string}`]: "error" | "warn" | "off";
};

const isESM = typeof require === "undefined";

// Base plugin structure, common across ESLint 8 and 9
const plugin = {
  configs: {},
  meta: {
    name: "@convex-dev/eslint-plugin",
    version: "0.0.0-alpha.0",
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
    /** Useful for custom convex directory locations */
    recommendedRulesCustomConvexDirectoryLocation: {
      rules: recommendedRules,
    },
  };

  // In CommonJS, we need to directly assign to module.exports
  module.exports = plugin;
}

// For ESM (ESLint 9)
export default plugin;
