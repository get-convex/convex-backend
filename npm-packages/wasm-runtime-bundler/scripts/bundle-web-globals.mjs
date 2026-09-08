import { build } from "esbuild";
import { mkdir, readdir } from "node:fs/promises";
import path from "node:path";

/**
 * Bundle the web-API globals the guest installs into a single script, and the
 * module trees `wasm_runtime`'s tests evaluate as ESM entries.
 *
 * Both land in `dist`, which `wasm_runtime`'s build script exposes as
 * `WASM_RUNTIME_BUNDLER_DIST_DIR`.
 */
async function main() {
  const packageRoot = path.resolve(import.meta.dirname, "..");
  const outDir = path.join(packageRoot, "dist");
  await mkdir(outDir, { recursive: true });

  const common = {
    absWorkingDir: packageRoot,
    mainFields: ["module", "main"],
    bundle: true,
    platform: "neutral",
    target: ["es2020"],
    nodePaths: [path.join(packageRoot, "node_modules")],
    logLevel: "silent",
  };

  await build({
    ...common,
    entryPoints: [path.join(packageRoot, "scripts", "web-globals.ts")],
    outfile: path.join(outDir, "web-globals.js"),
    format: "iife",
  });

  const fixturesDir = path.join(packageRoot, "scripts", "fixtures");
  const fixtures = (await readdir(fixturesDir)).filter((name) =>
    name.endsWith(".ts"),
  );
  await build({
    ...common,
    entryPoints: fixtures.map((name) => path.join(fixturesDir, name)),
    outdir: path.join(outDir, "fixtures"),
    format: "esm",
  });
}

await main();
