import convexPlugin from "@convex-dev/eslint-plugin";
import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default [
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...convexPlugin.configs.recommended,
  {
    ignores: ["dist/**", "node_modules/**", "convex/_generated/**"],
  },
];
