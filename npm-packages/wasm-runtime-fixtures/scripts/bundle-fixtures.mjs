import { build } from "esbuild";
import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { pathToFileURL } from "node:url";

function usage() {
  throw new Error(
    "usage: node scripts/bundle-fixtures.mjs [--out-dir <dir>] [fixture...]",
  );
}

// Node builtins the guest does not have, mapped to shims that are good enough
// for library code (such as `convex-test`) to run inside the Wasm runtime.
const NODE_SHIMS = {
  "node:async_hooks": path.join(
    import.meta.dirname,
    "shims",
    "node-async-hooks.mjs",
  ),
};

function convexRuntimePlugin(packageRoot) {
  const workspaceRequire = createRequire(import.meta.url);
  const isBareSpecifier = (specifier) => {
    if (specifier === "convex:runtime") {
      return false;
    }

    return (
      !specifier.startsWith(".") &&
      !specifier.startsWith("/") &&
      !specifier.startsWith("node:")
    );
  };

  return {
    name: "convex-runtime",
    setup(pluginBuild) {
      pluginBuild.onResolve({ filter: /^node:/ }, (args) => {
        const shim = NODE_SHIMS[args.path];
        return shim === undefined ? null : { path: shim };
      });

      pluginBuild.onResolve({ filter: /^convex:runtime$/ }, () => ({
        path: "convex:runtime",
        namespace: "convex-runtime",
      }));

      pluginBuild.onLoad({ filter: /.*/, namespace: "convex-runtime" }, () => ({
        contents: `
const runtime = () => {
  if (!globalThis.__convex_runtime) {
    throw new Error("convex:runtime is only available inside the Wasm guest runtime");
  }

  return globalThis.__convex_runtime;
};

export const db = {
  get: (key) => runtime().db.get(key),
  set: (key, value) => runtime().db.set(key, value),
  delete: (key) => runtime().db.delete(key),
};

export const console = {
  log: (...args) => runtime().console.log(...args),
  warn: (...args) => runtime().console.warn(...args),
  error: (...args) => runtime().console.error(...args),
};

export const crypto = {
  randomUUID: () => runtime().crypto.randomUUID(),
};

export const now = () => runtime().now();

export const sleep = (ms) => runtime().sleep(ms);
`,
        loader: "js",
      }));

      pluginBuild.onResolve({ filter: /.*/ }, (args) => {
        if (!isBareSpecifier(args.path)) {
          return null;
        }

        try {
          return {
            path: workspaceRequire.resolve(args.path, {
              paths: [args.resolveDir, packageRoot],
            }),
          };
        } catch {
          return null;
        }
      });
    },
  };
}

async function bundleFixture(packageRoot, fixture, outDir) {
  const fixtureDir = path.join(packageRoot, "fixtures", fixture);
  const entryPoint = path.join(fixtureDir, "src", "index.ts");
  const bundlePath = path.join(outDir, "bundle.mjs");
  const sourceMapPath = `${bundlePath}.map`;
  const guestBundlePath = path.join(outDir, "guest-bundle.js");
  const guestSourceMapPath = `${guestBundlePath}.map`;
  const manifestPath = path.join(outDir, "manifest.json");

  await mkdir(outDir, { recursive: true });

  const shared = {
    absWorkingDir: packageRoot,
    entryPoints: [entryPoint],
    bundle: true,
    nodePaths: [path.join(packageRoot, "node_modules")],
    platform: "neutral",
    target: ["es2020"],
    sourcemap: "external",
    logLevel: "silent",
    plugins: [convexRuntimePlugin(packageRoot)],
  };

  await build({ ...shared, outfile: bundlePath, format: "esm" });

  // The handler manifest is the set of exported functions, which esbuild does
  // not report, so read them off the ESM build's own module namespace. The
  // cache-busting query keeps repeated bundles of one fixture from resolving to
  // an earlier import of the same path.
  const moduleUrl = `${pathToFileURL(bundlePath).href}?cacheBust=${Date.now()}`;
  globalThis.__FELIX_BUNDLE_INTROSPECTION__ = true;
  const bundledModule = await import(moduleUrl);
  delete globalThis.__FELIX_BUNDLE_INTROSPECTION__;
  const handlers = Object.entries(bundledModule)
    .filter(([, value]) => typeof value === "function")
    .map(([name]) => name)
    .sort();

  const sourceMap = JSON.parse(await readFile(sourceMapPath, "utf8"));

  await build({
    ...shared,
    outfile: guestBundlePath,
    format: "iife",
    globalName: "__convex_exports",
  });

  const relative = (target) => path.relative(packageRoot, target);
  const manifest = {
    fixture,
    entryPoint: relative(entryPoint),
    bundle: relative(bundlePath),
    guestBundle: relative(guestBundlePath),
    guestSourceMap: relative(guestSourceMapPath),
    sourceMap: relative(sourceMapPath),
    handlers,
    sources: sourceMap.sources,
  };

  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
}

async function main() {
  const packageRoot = path.resolve(import.meta.dirname, "..");
  const args = process.argv.slice(2);
  const fixtures = [];
  let outDirArg;

  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--out-dir") {
      outDirArg = args[++i];
      if (outDirArg === undefined) {
        usage();
      }
    } else if (args[i].startsWith("-")) {
      usage();
    } else {
      fixtures.push(args[i]);
    }
  }

  if (fixtures.length === 0) {
    const entries = await readdir(path.join(packageRoot, "fixtures"), {
      withFileTypes: true,
    });
    fixtures.push(
      ...entries.filter((entry) => entry.isDirectory()).map(({ name }) => name),
    );
    fixtures.sort();
  }

  // `--out-dir` names the directory for a single fixture, so it only makes
  // sense for a single-fixture run; without it each fixture gets its own
  // subdirectory of the turbo-cached `dist`.
  if (outDirArg !== undefined && fixtures.length !== 1) {
    usage();
  }

  // esbuild's ESM output is imported back in for handler introspection, and two
  // fixtures bundling at once would race on `globalThis`.
  for (const fixture of fixtures) {
    const outDir =
      outDirArg === undefined
        ? path.join(packageRoot, "dist", fixture)
        : path.resolve(outDirArg);
    await bundleFixture(packageRoot, fixture, outDir);
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack : error);
  process.exitCode = 1;
});
