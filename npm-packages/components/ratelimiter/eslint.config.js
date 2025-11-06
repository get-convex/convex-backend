import globals from "globals";
import eslint from "@eslint/js";
import convexPlugin from "@convex-dev/eslint-plugin";

import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: [
      "dist",
      "*.config.{js,mjs,cjs,ts,tsx}",
      "**/_generated/",
      "node10stubs.mjs",
    ],
  },
  eslint.configs.recommended,
  {
    files: ["src/ratelimiter/**/*.ts"],
    plugins: {
      "@convex-dev": convexPlugin,
    },
    rules: convexPlugin.configs.recommended[0].rules,
  },
  ...tseslint.configs.recommended,

  {
    rules: {
      "@typescript-eslint/no-require-imports": "off",
      "@typescript-eslint/no-explicit-any": "off",
    },
  },
  {
    files: ["src/**/*.{js,mjs,cjs,ts,tsx}"],
    languageOptions: {
      parser: tseslint.parser,
      parserOptions: {
        project: ["tsconfig.json"],
        tsconfigRootDir: ".",
      },
    },
  },
  {
    files: ["src/**/*.{ts,tsx}"],
    ignores: ["src/react/**"],
    languageOptions: {
      globals: globals.worker,
    },
    rules: {
      "@typescript-eslint/no-floating-promises": "error",

      // allow (_arg: number) => {}
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
        },
      ],
    },
  },
);
