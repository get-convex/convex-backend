// @ts-check

import eslint from "@eslint/js";
import { defineConfig, globalIgnores } from "eslint/config";
import tseslint from "typescript-eslint";
import eslintPlugin from "eslint-plugin-eslint-plugin";

export default defineConfig(
  eslint.configs.recommended,
  tseslint.configs.recommended,
  eslintPlugin.configs.recommended,
  globalIgnores(["dist/**", "node_modules/**"]),
);
