import path from "path";
import { ESLintUtils } from "@typescript-eslint/utils";

// List of Convex function registrars to check for
export const CONVEX_REGISTRARS = [
  "query",
  "mutation",
  "action",
  "internalQuery",
  "internalMutation",
  "internalAction",
];

const ENTRY_POINT_EXTENSIONS = [
  // ESBuild js loader
  ".js",
  ".mjs",
  ".cjs",
  // ESBuild ts loader
  ".ts",
  ".tsx",
  ".mts",
  ".cts",
  // ESBuild jsx loader
  ".jsx",
  // ESBuild supports css, text, json, and more but these file types are not
  // allowed to define entry points.
];

/**
 * Assuming this is only called on files in the convex directory,
 * check return true if the file looks like an entry point.
 * This logic matches convex/src/bundler/index.ts.
 */
export function isEntryPoint(fpath: string) {
  const parsedPath = path.parse(fpath);
  const base = parsedPath.base;

  if (!ENTRY_POINT_EXTENSIONS.some((ext) => fpath.endsWith(ext))) {
    return false;
  } else if (fpath.includes("_generated" + path.sep)) {
    return false;
  } else if (base.startsWith(".")) {
    return false;
  } else if (base.startsWith("#")) {
    return false;
  } else if (base === "schema.ts" || base === "schema.js") {
    return false;
  } else if ((base.match(/\./g) || []).length > 1) {
    return false;
  } else if (fpath.includes(" ")) {
    return false;
  } else {
    return true;
  }
}

export const createRule = ESLintUtils.RuleCreator(
  (name) => `https://docs.convex.dev/eslint#${name}`,
);
