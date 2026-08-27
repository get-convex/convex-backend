import { build } from "esbuild";
import { mkdir } from "node:fs/promises";
import path from "node:path";

/**
 * Bundle the web-API globals the guest installs into a single script.
 *
 * The output lands beside the fixture bundles so that `wasm_runtime`'s
 * build script can hand it to the compiler through
 * `WASM_RUNTIME_FIXTURE_BUNDLES_DIR`.
 */
async function main() {
  const packageRoot = path.resolve(import.meta.dirname, "..");
  const outDir = path.join(packageRoot, "dist");
  await mkdir(outDir, { recursive: true });

  await build({
    absWorkingDir: packageRoot,
    entryPoints: [path.join(packageRoot, "scripts", "web-globals.ts")],
    outfile: path.join(outDir, "web-globals.js"),
    bundle: true,
    format: "iife",
    platform: "neutral",
    target: ["es2020"],
    nodePaths: [path.join(packageRoot, "node_modules")],
    logLevel: "silent",
  });
}

await main();
