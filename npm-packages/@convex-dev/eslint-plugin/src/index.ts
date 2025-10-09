import { noImportUseNode } from "./lib/noImportUseNode.js";
import noOldRegisteredFunctionSyntax from "./lib/noOldRegisteredFunctionSyntax.js";
import { noMissingArgs, noArgsWithoutValidator } from "./lib/noMissingArgs.js";
import { RuleModule } from "@typescript-eslint/utils/ts-eslint";
import { version } from "./version.js";

const rules = {
  "no-old-registered-function-syntax": noOldRegisteredFunctionSyntax,
  "no-args-without-validator": noArgsWithoutValidator,
  "no-missing-args-validator": noMissingArgs,
  "import-wrong-runtime": noImportUseNode,
} satisfies Record<string, RuleModule<string, unknown[]>>;

const recommendedRules = {
  // This rule is a good idea but bothersome to convert projects to later:
  // it's possible to safely import specific exports from a "use node"
  // file if all Node.js-specific imports are side-effect free.
  "@convex-dev/import-wrong-runtime": "off",
  "@convex-dev/no-old-registered-function-syntax": "error",
  // This is a reasonable idea in large projects: throw at runtime
  // when API endpoints that don't expect arguments receive them.
  // But it lacks the typical benefit of a validator providing
  // types so it feels more pedantic.
  "@convex-dev/no-missing-args-validator": "off",
  "@convex-dev/no-args-without-validator": "error",
} satisfies {
  [key: `@convex-dev/${string}`]: "error" | "warn" | "off";
};

const isESM = typeof require === "undefined";

// Base plugin structure, common across ESLint 8 and 9
const plugin = {
  // loose types so this can work with ESlint 8 and 9
  configs: {} as {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    recommended: any;
    /** Only available in ESlint 8 */
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    recommendedRulesCustomConvexDirectoryLocation: any;
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
