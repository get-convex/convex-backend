import { defineConfig, globalIgnores } from "eslint/config";
import path from "node:path";
import js from "@eslint/js";
import { fileURLToPath } from "node:url";
import { FlatCompat } from "@eslint/eslintrc";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const compat = new FlatCompat({
  baseDirectory: __dirname,
  recommendedConfig: js.configs.recommended,
  allConfig: js.configs.all,
});

export default defineConfig([
  {
    extends: compat.extends(
      "plugin:@typescript-eslint/recommended",
      "plugin:@convex-dev/recommended",
    ),
  },
  globalIgnores([
    "**/node_modules/",
    "**/dist/",
    "**/build/",
    "convex/_generated/**",
  ]),
]);
