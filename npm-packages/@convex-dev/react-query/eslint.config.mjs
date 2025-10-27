import { defineConfig, globalIgnores } from "eslint/config";
import { fixupConfigRules } from "@eslint/compat";
import path from "node:path";
import { fileURLToPath } from "node:url";
import js from "@eslint/js";
import { FlatCompat } from "@eslint/eslintrc";
import convexPlugin from "@convex-dev/eslint-plugin";
import tseslint from "typescript-eslint";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const compat = new FlatCompat({
  baseDirectory: __dirname,
  recommendedConfig: js.configs.recommended,
  allConfig: js.configs.all,
});

export default defineConfig([
  {
    extends: fixupConfigRules(
      compat.extends(
        "plugin:react/jsx-runtime",
        "plugin:react-hooks/recommended",
        "plugin:@tanstack/eslint-plugin-query/recommended",
      ),
    ),
  },
  globalIgnores(["convex/_generated/**", "dist"]),
  ...tseslint.configs.recommended,
  ...convexPlugin.configs.recommended,
  {
    rules: {
      // allow (_arg: number) => {}
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
        },
      ],

      "@typescript-eslint/no-explicit-any": "off",
      "@typescript-eslint/no-empty-object-type": "off",
    },
  },
]);
