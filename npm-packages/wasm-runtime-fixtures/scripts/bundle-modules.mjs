import { build } from "esbuild";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

function usage() {
  throw new Error(
    "usage: node scripts/bundle-modules.mjs <source-dir> <entry-module> <out-dir>",
  );
}

/**
 * Bundle a deployment's module tree into a single script the guest can
 * evaluate.
 *
 * The sources are what `convex push` already ran through esbuild, so every
 * import inside them is a relative path to another file in the same tree and
 * nothing needs resolving from node_modules. A bare specifier means the tree is
 * not self-contained, and esbuild is left to fail on it rather than being given
 * a resolver that might paper over it.
 */
async function main() {
  const [sourceArg, entryArg, outArg] = process.argv.slice(2);
  if (!sourceArg || !entryArg || !outArg) {
    usage();
  }

  const sourceDir = path.resolve(sourceArg);
  const entryPoint = path.resolve(sourceDir, entryArg);
  const outDir = path.resolve(outArg);
  const guestBundlePath = path.join(outDir, "guest-bundle.js");
  const manifestPath = path.join(outDir, "manifest.json");

  await mkdir(outDir, { recursive: true });

  const result = await build({
    absWorkingDir: sourceDir,
    entryPoints: [entryPoint],
    outfile: guestBundlePath,
    bundle: true,
    format: "iife",
    globalName: "__convex_exports",
    platform: "neutral",
    target: ["es2020"],
    sourcemap: "external",
    metafile: true,
    logLevel: "silent",
  });

  const [entryOutput] = Object.values(result.metafile.outputs).filter(
    (output) => output.entryPoint !== undefined,
  );

  const manifest = {
    // Which global surface the guest installs before evaluating this bundle.
    // See `bundle_kind` in `guest_js/src/lib.rs`.
    kind: "udf",
    entryPoint: path.relative(sourceDir, entryPoint),
    guestBundle: path.basename(guestBundlePath),
    guestSourceMap: `${path.basename(guestBundlePath)}.map`,
    exports: (entryOutput?.exports ?? []).sort(),
    inputs: Object.keys(result.metafile.inputs).sort(),
  };

  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
}

await main();
