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
    files: ["**/convex/**/*.ts"],
    languageOptions: {
      parser: tseslint.parser,
      parserOptions: {
        project: ["convex/tsconfig.json"],
        tsconfigRootDir: ".",
      },
    },
    rules: {
      "@typescript-eslint/no-floating-promises": "error",
      "@convex-dev/require-args-validator": [
        "error",
        {
          ignoreUnusedArguments: true,
        },
      ],
    },
  },
);
