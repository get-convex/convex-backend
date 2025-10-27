import { defineConfig } from "eslint/config";

import tsParser from "@typescript-eslint/parser";
import typescriptEslint from "@typescript-eslint/eslint-plugin";
import reactHooks from "eslint-plugin-react-hooks";
import react from "eslint-plugin-react";
import jest from "eslint-plugin-jest";
import convexPlugin from "@convex-dev/eslint-plugin";

import { fixupPluginRules } from "@eslint/compat";

import globals from "globals";
import js from "@eslint/js";

import { FlatCompat } from "@eslint/eslintrc";
import { fileURLToPath } from "node:url";
import path from "node:path";

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
      "@typescript-eslint/no-explicit-any": "off",
      "@typescript-eslint/no-non-null-assertion": "off",
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",
      eqeqeq: ["error", "always"],
      "@typescript-eslint/no-var-requires": "off",
      "jest/expect-expect": "off",
      "jest/no-conditional-expect": "off",
      "@typescript-eslint/no-unused-vars": "off",
      "@typescript-eslint/no-empty-function": "off",
    },
  },
  ...convexPlugin.configs.recommended,
  {
    ignores: ["convex/_generated/**"],
  },
]);
