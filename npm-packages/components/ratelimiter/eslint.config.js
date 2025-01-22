import eslint from "@eslint/js";
import tseslint from "typescript-eslint";
import convex from "@convex-dev/eslint-plugin";

export default tseslint.config(
  {
    ignores: ["dist", "convex/_generated/**"],
  },
  eslint.configs.recommended,
  ...convex.configs.recommended,
  ...tseslint.configs.recommended,

  {
    rules: {
      "@typescript-eslint/no-require-imports": "off",
      "@typescript-eslint/no-explicit-any": "off",
    },
  },
  {
    files: ["src/ratelimiter/**/*.ts"],
    languageOptions: {
      parser: tseslint.parser,
      parserOptions: {
        project: ["tsconfig.json"],
        tsconfigRootDir: ".",
      },
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
