// We use an `.mjs` file instead of TypeScript so node can run the script directly.
import { bundle, entryPointsByEnvironment } from "../dist/esm/bundler/index.js";
import { oneoffContext } from "../dist/esm/bundler/context.js";
import path from "path";

if (process.argv.length < 3) {
  throw new Error(
    "USAGE: node bundle-server.mjs <udf system dir> <system dir>*",
  );
}
const systemDirs = process.argv.slice(3);
const out = [];

// Only bundle "setup.ts" from `udf/_system`.
const udfDir = process.argv[2];
const setupPath = path.join(udfDir, "setup.ts");
const setupBundles = (
  await bundle(oneoffContext, process.argv[2], [setupPath], true, "browser")
).modules;
if (setupBundles.length !== 1) {
  throw new Error("Got more than one setup bundle?");
}
out.push(...setupBundles);

for (const systemDir of systemDirs) {
  if (path.basename(systemDir) !== "_system") {
    throw new Error(`Refusing to bundle non-system directory ${systemDir}`);
  }
  const entryPoints = await entryPointsByEnvironment(
    oneoffContext,
    systemDir,
    false,
  );
  const bundles = (
    await bundle(
      oneoffContext,
      systemDir,
      entryPoints.isolate,
      false,
      "browser",
    )
  ).modules;
  out.push(...bundles);
}
process.stdout.write(JSON.stringify(out));
