import { defineConfig, globalIgnores } from "eslint/config";

import eslint from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import react from "eslint-plugin-react";
import tseslint, { parser as tsParser } from "typescript-eslint";
import convexPlugin from "@convex-dev/eslint-plugin";

import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import { fixupPluginRules } from "@eslint/compat";

import js from "@eslint/js";

import { FlatCompat } from "@eslint/eslintrc";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const compat = new FlatCompat({
  baseDirectory: __dirname,
  recommendedConfig: js.configs.recommended,
  allConfig: js.configs.all,
});

export default defineConfig([
  eslint.configs.recommended,
  tseslint.configs.recommended,
  ...convexPlugin.configs.recommended,
  {
    languageOptions: {
      parser: tsParser,

      parserOptions: {
        project: join(__dirname, "tsconfig.json"),
        tsconfigRootDir: __dirname,
      },
    },

    plugins: {
      "react-hooks": fixupPluginRules(reactHooks),
      react,
    },

    extends: compat.extends("prettier"),

    rules: {
      // we use any to access internal-only APIs in Convex functions
      "@typescript-eslint/no-explicit-any": "off",

      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          varsIgnorePattern: "_.*",
        },
      ],

      "@typescript-eslint/no-floating-promises": "error",

      // system UDF argument validation
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
    "**/node_modules",
    "**/dist",
    "**/_generated",
    "eslint.config.mjs",
  ]),
]);
