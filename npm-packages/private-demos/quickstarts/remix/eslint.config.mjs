import { defineConfig, globalIgnores } from "eslint/config";

import globals from "globals";

import { fixupConfigRules, fixupPluginRules } from "@eslint/compat";

import _import from "eslint-plugin-import";
import tsParser from "@typescript-eslint/parser";
import js from "@eslint/js";

import convexPlugin from "@convex-dev/eslint-plugin";

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
      ecmaVersion: "latest",
      sourceType: "module",

      parserOptions: {
        ecmaFeatures: {
          jsx: true,
        },
      },

      globals: {
        ...globals.browser,
        ...globals.commonjs,
      },
    },

    extends: compat.extends("eslint:recommended"),
  },
  ...convexPlugin.configs.recommended,
  globalIgnores(["!**/.server", "!**/.client", "convex/_generated/**"]),
  {
    files: ["**/*.{js,jsx,ts,tsx}"],

    extends: fixupConfigRules(
      compat.extends(
        "plugin:react/recommended",
        "plugin:react/jsx-runtime",
        "plugin:react-hooks/recommended",
        "plugin:jsx-a11y/recommended",
      ),
    ),

    settings: {
      react: {
        version: "detect",
      },

      formComponents: ["Form"],

      linkComponents: [
        {
          name: "Link",
          linkAttribute: "to",
        },
        {
          name: "NavLink",
          linkAttribute: "to",
        },
      ],

      "import/resolver": {
        typescript: {},
      },
    },
  },
  {
    files: ["**/*.{ts,tsx}"],

    plugins: {
      import: fixupPluginRules(_import),
    },

    languageOptions: {
      parser: tsParser,
    },

    settings: {
      "import/internal-regex": "^~/",

      "import/resolver": {
        node: {
          extensions: [".ts", ".tsx"],
        },

        typescript: {
          alwaysTryTypes: true,
        },
      },
    },

    extends: fixupConfigRules(
      compat.extends(
        "plugin:@typescript-eslint/recommended",
        "plugin:import/recommended",
        "plugin:import/typescript",
      ),
    ),
  },
  {
    files: ["**/.eslintrc.cjs"],

    languageOptions: {
      globals: {
        ...globals.node,
      },
    },
  },
]);
