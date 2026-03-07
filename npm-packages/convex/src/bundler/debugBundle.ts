import path from "path";
import esbuild, { BuildFailure, LogLevel, Plugin } from "esbuild";
import { Context } from "./context.js";
import {
  logError,
  changeSpinner,
  logFailure,
  logVerbose,
  logMessage,
} from "./log.js";
import { wasmPlugin } from "./wasm.js";
import dependencyTrackerPlugin from "./depgraph.js";

export async function innerEsbuild({
  entryPoints,
  platform,
  dir,
  extraConditions,
  generateSourceMaps,
  plugins,
  chunksFolder,
  logLevel,
  includeSourcesContent = false,
  splitting = true,
}: {
  entryPoints: string[];
  platform: esbuild.Platform;
  dir: string;
  extraConditions: string[];
  generateSourceMaps: boolean;
  plugins: Plugin[];
  chunksFolder: string;
  logLevel?: LogLevel;
  includeSourcesContent?: boolean;
  splitting?: boolean | undefined;
}) {
  // In isolate bundles, resolve selected Node.js built-ins to inline shims.
  const nodeShimsPlugin: esbuild.Plugin = {
    name: "convex-node-shims",
    setup(build) {
      if (platform !== "browser") return;

      // async_hooks / node:async_hooks
      const asyncHooksFilter = /^(node:)?async_hooks$/;
      build.onResolve({ filter: asyncHooksFilter }, (args) => ({
        path: args.path,
        namespace: "async-hooks-shim",
      }));
      build.onLoad({ filter: /.*/, namespace: "async-hooks-shim" }, () => ({
        contents: `
            const m = globalThis.__async_hooks__;
            export const AsyncLocalStorage = globalThis.AsyncLocalStorage;
            export const AsyncResource = globalThis.AsyncResource;
            export const createHook = m ? m.createHook : () => ({ enable() { return this; }, disable() { return this; } });
            export const executionAsyncId = m ? m.executionAsyncId : () => 0;
            export const triggerAsyncId = m ? m.triggerAsyncId : () => 0;
            export const executionAsyncResource = m ? m.executionAsyncResource : () => ({});
            export const asyncWrapProviders = m ? m.asyncWrapProviders : {};
            export default { AsyncLocalStorage, AsyncResource, createHook, executionAsyncId, triggerAsyncId, executionAsyncResource, asyncWrapProviders };
          `,
        loader: "js",
      }));

      // events / node:events
      const eventsFilter = /^(node:)?events$/;
      build.onResolve({ filter: eventsFilter }, (args) => ({
        path: args.path,
        namespace: "events-shim",
      }));
      build.onLoad({ filter: /.*/, namespace: "events-shim" }, () => ({
        contents: `
            class EventEmitter {
              constructor() { this._listeners = {}; }
              on(event, fn) { return this.addListener(event, fn); }
              addListener(event, fn) {
                if (!this._listeners[event]) this._listeners[event] = [];
                this._listeners[event].push(fn);
                return this;
              }
              once(event, fn) {
                const wrapped = (...args) => { this.removeListener(event, wrapped); fn(...args); };
                return this.addListener(event, wrapped);
              }
              off(event, fn) { return this.removeListener(event, fn); }
              removeListener(event, fn) {
                if (this._listeners[event]) {
                  this._listeners[event] = this._listeners[event].filter(l => l !== fn);
                }
                return this;
              }
              removeAllListeners(event) {
                if (event) { delete this._listeners[event]; }
                else { this._listeners = {}; }
                return this;
              }
              emit(event, ...args) {
                const fns = this._listeners[event];
                if (!fns || fns.length === 0) return false;
                fns.slice().forEach(fn => fn(...args));
                return true;
              }
              listenerCount(event) { return (this._listeners[event] || []).length; }
              listeners(event) { return (this._listeners[event] || []).slice(); }
              eventNames() { return Object.keys(this._listeners); }
              prependListener(event, fn) {
                if (!this._listeners[event]) this._listeners[event] = [];
                this._listeners[event].unshift(fn);
                return this;
              }
              prependOnceListener(event, fn) {
                const wrapped = (...args) => { this.removeListener(event, wrapped); fn(...args); };
                return this.prependListener(event, wrapped);
              }
              setMaxListeners() { return this; }
              getMaxListeners() { return 10; }
              rawListeners(event) { return this.listeners(event); }
            }
            export { EventEmitter };
            export default EventEmitter;
          `,
        loader: "js",
      }));
    },
  };

  const result = await esbuild.build({
    entryPoints,
    bundle: true,
    platform: platform,
    format: "esm",
    target: "esnext",
    jsx: "automatic",
    outdir: "out",
    outbase: dir,
    conditions: ["convex", "module", ...extraConditions],
    plugins: [nodeShimsPlugin, ...plugins],
    write: false,
    sourcemap: generateSourceMaps,
    sourcesContent: includeSourcesContent,
    splitting,
    chunkNames: path.join(chunksFolder, "[hash]"),
    treeShaking: true,
    minifySyntax: true,
    minifyIdentifiers: true,
    // Enabling minifyWhitespace breaks sourcemaps on convex backends.
    // The sourcemaps produced are valid on https://evanw.github.io/source-map-visualization
    // but something we're doing (perhaps involving https://github.com/getsentry/rust-sourcemap)
    // makes everything map to the same line.
    minifyWhitespace: false, // false is the default, just showing for clarify.
    keepNames: true,
    define: {
      "process.env.NODE_ENV": '"production"',
    },
    metafile: true,
    logLevel: logLevel || "warning",
  });
  return result;
}

export function isEsbuildBuildError(e: any): e is BuildFailure {
  return (
    "errors" in e &&
    "warnings" in e &&
    Array.isArray(e.errors) &&
    Array.isArray(e.warnings)
  );
}

/**
 * Bundle non-"use node" entry points one at a time to track down the first file with an error
 * is being imported.
 */
export async function debugIsolateBundlesSerially(
  ctx: Context,
  {
    entryPoints,
    extraConditions,
    dir,
  }: {
    entryPoints: string[];
    extraConditions: string[];
    dir: string;
  },
): Promise<void> {
  logMessage(
    `Bundling convex entry points one at a time to track down things that can't be bundled for the Convex JS runtime.`,
  );
  let i = 1;
  for (const entryPoint of entryPoints) {
    changeSpinner(
      `bundling entry point ${entryPoint} (${i++}/${entryPoints.length})...`,
    );

    const { plugin, tracer } = dependencyTrackerPlugin();
    try {
      await innerEsbuild({
        entryPoints: [entryPoint],
        platform: "browser",
        generateSourceMaps: true,
        chunksFolder: "_deps",
        extraConditions,
        dir,
        plugins: [plugin, wasmPlugin],
        logLevel: "silent",
      });
    } catch (error) {
      if (!isEsbuildBuildError(error) || !error.errors[0]) {
        return await ctx.crash({
          exitCode: 1,
          errorType: "invalid filesystem data",
          printedMessage: null,
        });
      }

      const buildError = error.errors[0];
      const errorFile = buildError.location?.file;
      if (!errorFile) {
        return await ctx.crash({
          exitCode: 1,
          errorType: "invalid filesystem data",
          printedMessage: null,
        });
      }

      const importedPath = buildError.text.match(/"([^"]+)"/)?.[1];
      if (!importedPath) continue;

      const full = path.resolve(errorFile);
      logError("");
      logError(
        `Bundling ${entryPoint} resulted in ${error.errors.length} esbuild errors.`,
      );
      logError(`One of the bundling errors occurred while bundling ${full}:\n`);
      logError(
        esbuild
          .formatMessagesSync([buildError], {
            kind: "error",
            color: true,
          })
          .join("\n"),
      );
      logError("It would help to avoid importing this file.");
      const chains = tracer.traceImportChains(entryPoint, full);
      const chain: string[] = chains[0];
      chain.reverse();

      logError(``);
      if (chain.length > 0) {
        const problematicFileRelative = formatFilePath(dir, chain[0]);

        if (chain.length === 1) {
          logError(`  ${problematicFileRelative}`);
        } else {
          logError(`  ${problematicFileRelative} is imported by`);

          for (let i = 1; i < chain.length - 1; i++) {
            const fileRelative = formatFilePath(dir, chain[i]);
            logError(`  ${fileRelative}, which is imported by`);
          }

          const entryPointFile = chain[chain.length - 1];
          const entryPointRelative = formatFilePath(dir, entryPointFile);

          logError(`  ${entryPointRelative}, which doesn't use "use node"\n`);
          logError(
            `  For registered action functions to use Node.js APIs in any code they run they must be defined\n` +
              `  in a file with 'use node' at the top. See https://docs.convex.dev/functions/runtimes#nodejs-runtime\n`,
          );
        }
      }

      logFailure("Bundling failed");
      return await ctx.crash({
        exitCode: 1,
        errorType: "invalid filesystem data",
        printedMessage: "Bundling failed.",
      });
    }
    logVerbose(`${entryPoint} bundled`);
  }
}

// Helper function to format file paths consistently
function formatFilePath(baseDir: string, filePath: string): string {
  // If it's already a relative path like "./shared", just return it
  if (!path.isAbsolute(filePath)) {
    // For relative paths, ensure they start with "convex/"
    if (!filePath.startsWith("convex/")) {
      // If it's a path like "./subdir/file.ts" or "subdir/file.ts"
      const cleanPath = filePath.replace(/^\.\//, "");
      return `convex/${cleanPath}`;
    }
    return filePath;
  }

  // Get the path relative to the base directory
  const relativePath = path.relative(baseDir, filePath);

  // Remove any leading "./" that path.relative might add
  const cleanPath = relativePath.replace(/^\.\//, "");

  // Check if this is a path within the convex directory
  const isConvexPath =
    cleanPath.startsWith("convex/") ||
    cleanPath.includes("/convex/") ||
    path.dirname(cleanPath) === "convex";

  if (isConvexPath) {
    // If it already starts with convex/, return it as is
    if (cleanPath.startsWith("convex/")) {
      return cleanPath;
    }

    // For files in the convex directory
    if (path.dirname(cleanPath) === "convex") {
      const filename = path.basename(cleanPath);
      return `convex/${filename}`;
    }

    // For files in subdirectories of convex
    const convexIndex = cleanPath.indexOf("convex/");
    if (convexIndex >= 0) {
      return cleanPath.substring(convexIndex);
    }
  }

  // For any other path, assume it's in the convex directory
  // This handles cases where the file is in a subdirectory of convex
  // but the path doesn't include "convex/" explicitly
  return `convex/${cleanPath}`;
}
