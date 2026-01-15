import { defineConfig, globalIgnores } from "eslint/config";

import tsParser from "@typescript-eslint/parser";
import typescriptEslint from "@typescript-eslint/eslint-plugin";
import reactHooks from "eslint-plugin-react-hooks";
import react from "eslint-plugin-react";
import jest from "eslint-plugin-jest";
import eslintPlugin from "eslint-plugin-eslint-plugin";
import convexPlugin from "@convex-dev/eslint-plugin";

import { fixupPluginRules } from "@eslint/compat";

import globals from "globals";
import js from "@eslint/js";

import { FlatCompat } from "@eslint/eslintrc";

import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const compat = new FlatCompat({
  baseDirectory: __dirname,
  recommendedConfig: js.configs.recommended,
  allConfig: js.configs.all,
});

export default defineConfig([
  {
    languageOptions: {
      parser: tsParser,

      globals: {
        ...globals.amd,
        ...globals.browser,
        ...globals.jest,
        ...globals.node,
      },
    },

    plugins: {
      "@typescript-eslint": typescriptEslint,
      "react-hooks": fixupPluginRules(reactHooks),
      react,
      jest,
    },

    extends: compat.extends(
      "eslint:recommended",
      "plugin:@typescript-eslint/recommended",
      "prettier",
      "plugin:jest/recommended",
    ),

    rules: {
      // any is terrible but we use it a lot (even in our public code).
      "@typescript-eslint/no-explicit-any": "off",

      // asserting that values aren't null is risky but useful.
      "@typescript-eslint/no-non-null-assertion": "off",

      // Add React hooks rules so we don't misuse them.
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",

      eqeqeq: ["error", "always"],

      // allow (_arg: number) => {}
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
        },
      ],

      "jest/expect-expect": "off",
      "jest/no-conditional-expect": "off",
    },
  },

  // Disable `console` except in packages where we need it
  {
    rules: {
      "no-console": "error",
    },
    basePath: path.join(__dirname, "..", ".."),
    ignores: [
      "**/dont-publish-alpha-as-latest.mjs",
      "**/version-check.mjs",
      "npm-packages-private/data/**",
      "npm-packages-private/postalservice/**",
      "npm-packages/@convex-dev/codemod/**",
      "npm-packages/component-tests/**",
      "npm-packages/convex-analytics/**",
      "npm-packages/convex-chat-speculative/**",
      "npm-packages/demos/**",
      "npm-packages/docs/**",
      "npm-packages/js-integration-tests/**",
      "npm-packages/local-store/**",
      "npm-packages/node-executor/**",
      "npm-packages/private-demos/**",
      "npm-packages/publishing-tests/**",
      "npm-packages/retention-tester/**",
      "npm-packages/scenario-runner/**",
      "npm-packages/shared-cursors/**",
      "npm-packages/simulation/**",
      "npm-packages/text-importer/**",
      "npm-packages/udf-runtime/**",
      "npm-packages/udf-tests/**",
      "npm-packages/version/**",
      "npm-packages/components/ratelimiter/node10stubs.mjs",
    ],
  },

  // Allow CJS imports in `.js` and `.cjs` files
  {
    files: ["**/*.js", "**/*.cjs"],
    rules: {
      "@typescript-eslint/no-require-imports": "off",
    },
  },

  // Set-up typescript-eslint rules that need a project configuration
  ...[
    "@convex-dev/eslint-plugin",
    "js-integration-tests",
    "scenario-runner/convex",
    "system-udfs",
    "udf-tests/convex",
    "components/ratelimiter",
    // FIXME: Ideally we’d add many more packages here
  ].map((pkg) => ({
    files: [path.join(pkg, "**/*.ts"), path.join(pkg, "**/*.tsx")],
    rules: {
      "@typescript-eslint/no-floating-promises": "error",
    },
    languageOptions: {
      parserOptions: {
        project: path.join(__dirname, "..", pkg, "tsconfig.json"),
      },
    },
  })),

  {
    files: ["**/convex/**/*.{js,ts}", "components/**/*.ts"],
    ignores: [
      // Some tests rely on using the old Convex function syntax, so we disable
      // the linter on test files
      "js-integration-tests/**",
      "udf-tests/**",

      // TODO(nicolas) Lint Postalservice too
      "postalservice/**",
    ],
    plugins: {
      "@convex-dev": convexPlugin,
    },
    rules: convexPlugin.configs.recommended[0].rules,
  },

  // @convex-dev/eslint-plugin: lint with eslint-plugin-eslint-plugin
  {
    files: ["@convex-dev/eslint-plugin/src/**/*.ts"],
    ...eslintPlugin.configs.recommended,
  },

  // system-udfs
  {
    files: ["system-udfs/**/*.ts"],
    rules: {
      // TODO(nicolas): use the new `ctx.db` APIs in system-udfs
      "@convex-dev/explicit-table-ids": "off",

      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["**/_generated/server"],

              importNames: [
                "query",
                "mutation",
                "action",
                "internalQuery",
                "internalMutation",
                "internalAction",
              ],

              message:
                "Use the query wrappers from convex/_system/server.ts instead for system UDF argument validation.",
            },
            {
              group: ["convex/server"],

              importNames: [
                "queryGeneric",
                "mutationGeneric",
                "actionGeneric",
                "internalQueryGeneric",
                "internalMutationGeneric",
                "internalActionGeneric",
              ],

              message:
                "Use the query wrappers from convex/_system/server.ts instead for system UDF argument validation.",
            },
          ],
        },
      ],
    },
  },

  globalIgnores([
    "**/.next/**",
    "**/.nuxt/**",
    "**/node_modules",
    "**/dist",
    "**/.next",
    "**/.nuxt",
    "**/build",
    "common/deploy",
    "common/scripts",
    "common/temp",
    "convex", // has a similar config, separate so vscode can find it
    "@convex-dev/react-query", // separate because OSS
    "@convex-dev/design-system", // has its own config using different dependencies
    "dashboard", // has its own config using different dependencies
    "dashboard-common", // has its own config using different dependencies
    "dashboard-self-hosted", // has its own config using different dependencies
    "dashboard-storybook", // has its own config using different dependencies
    "docs/.docusaurus", // auto-generated by Docusaurus
    "demos/nextjs-pages-router", // has its own config using different dependencies
    "demos/nextjs-app-router", // has its own config using different dependencies
    "private-demos/react-native", // has its own config using different dependencies
    "private-demos/actions", // has its own ESLint
    "private-demos/npm-showcase", // has its own ESLint and `"@typescript-eslint"`
    "private-demos/quickstarts/nodejs/script.js", // uses require
    "private-demos/quickstarts/nextjs-app-dir", // should match Next.js quickstart
    "private-demos/quickstarts/nextjs-app-dir-14", // should match Next.js quickstart
    "private-demos/nextjs-app-router-snippets", // should match Next.js quickstart
    "private-demos/nextjs-15-app", // should match Next.js quickstart
    "private-demos/nextjs-15-app-clerk", // should match Next.js quickstart
    "private-demos/quickstarts/sveltekit", // sveltekit linting is annoying to set up
    "private-demos/quickstarts/remix", // won't have these deps installed
    "private-demos/quickstarts/vue", // won't have these deps installed
    "private-demos/snippets", // has its own config
    "private-demos/tanstack-start", // has its own config
    "private-demos/tanstack-start-clerk", // has its own config
    "private-demos/tutorial/src/App.tsx", // tutorial warning will go away once user does tutorial
    "demos/html/script.js", // uses js-doc
    "demos/html/browser.bundle.js", // just until we can link to a CDN for this
    "create-convex/template-*",
    "convex-ai-chat/esm",
    "convex-ai-chat/cjs",
    "**/_generated/**", // auto-generated files
  ]),
]);
