import { defineConfig } from "tsdown";
import { readFileSync } from "node:fs";

const packageVersion =
  process.env.npm_package_version ??
  JSON.parse(readFileSync("./package.json", "utf-8")).version;

export default defineConfig({
  entry: ["src/index.ts"],
  exports: true,
  format: "esm",
  dts: {
    tsconfig: "./tsconfig.json",
    sourcemap: true,
  },
  sourcemap: true,
  clean: true, // (this is the default)
  // Compiled-in variables
  env: {
    npm_package_version: packageVersion,
  },
});
