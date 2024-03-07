#!/usr/bin/env node
/*
 * Check that dependencies only used by the CLI are not present in package.json dependencies.
 */

import depcheck from "depcheck";
import process from "process";
import path from "path";
import * as url from "url";

const __dirname = url.fileURLToPath(new URL(".", import.meta.url));
const root = path.dirname(__dirname);

const options = {
  ignorePatterns: [
    "dist",
    "src/cli", // CLI deps are bundled, they use devDependencies
    "src/bundler", // Bundler is only used by the CLI
  ],
  ignoreMatches: [
    "esbuild", // the only unbundled dependency of the CLI
  ],
};

depcheck(root, options).then((unused) => {
  if (unused.dependencies.length) {
    console.log(
      "Some package.json dependencies are only used in CLI (or not at all):",
    );
    console.log(
      "If a dependency is only used in the CLI, add it to devDependencies instead.",
    );
    console.log(unused.dependencies);
    process.exit(1);
  }
});
