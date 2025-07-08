import { existsSync, readFileSync } from "fs";
import { join } from "path";
import { Context } from "./context";
import chalk from "chalk";
import semver from "semver";

export async function findTsconfig(
  ctx: Context,
  root: string,
): Promise<string> {
  const defaultConvexRoot = join(root, "convex");

  let convexFile: string | undefined = undefined;
  try {
    convexFile = readFileSync(join(root, "convex.json"), "utf-8");
  } catch {
    return await findTsconfigInConvexRoot(ctx, defaultConvexRoot);
  }

  let convexFileContents;
  try {
    convexFileContents = JSON.parse(convexFile);
  } catch (err) {
    return await ctx.crash({
      printedMessage: `Invalid \`convex.json\` file (in ${root}): ${err}`,
    });
  }

  if (typeof convexFileContents !== "object") {
    return await ctx.crash({
      printedMessage: `Invalid \`convex.json\` file (in ${root}): not an object`,
    });
  }

  if (
    "functions" in convexFileContents &&
    typeof convexFileContents.functions !== "string"
  ) {
    return await ctx.crash({
      printedMessage: `Invalid \`convex.json\` file (in ${root}): \`functions\` must be a string`,
    });
  }

  return await findTsconfigInConvexRoot(
    ctx,
    "functions" in convexFileContents
      ? join(root, convexFileContents.functions)
      : defaultConvexRoot,
  );
}

async function findTsconfigInConvexRoot(
  ctx: Context,
  convexRoot: string,
): Promise<string> {
  const tsconfigPath = join(convexRoot, "tsconfig.json");

  if (!existsSync(tsconfigPath)) {
    return await ctx.crash({
      printedMessage: `No tsconfig.json found in ${convexRoot}`,
    });
  }

  return tsconfigPath;
}

const convexCheckHint = `\n${chalk.gray("Tip: You can run the codemod with --skip-convex-check to ignore this check.")}`;

export async function checkConvexVersion(
  ctx: Context,
  root: string,
  semverRange: string,
) {
  let packageJsonContents;
  try {
    packageJsonContents = readFileSync(join(root, "package.json"), "utf-8");
  } catch (err) {
    return await ctx.crash({
      printedMessage: `Can’t read package.json (in ${root}): ${err}\nPlease make sure you are running the codemod from the root of your project (or use --root to specify a different folder).${convexCheckHint}`,
    });
  }

  await checkConvexVersionFromPackageJson(
    ctx,
    root,
    packageJsonContents,
    semverRange,
  );
}

export async function checkConvexVersionFromPackageJson(
  ctx: Context,
  root: string,
  packageJsonContents: string,
  semverRange: string,
) {
  let convexFileContents;
  try {
    convexFileContents = JSON.parse(packageJsonContents);
  } catch (err) {
    return await ctx.crash({
      printedMessage: `Can’t parse package.json (in ${root}): ${err}${convexCheckHint}`,
    });
  }

  if (typeof convexFileContents !== "object") {
    return await ctx.crash({
      printedMessage: `Invalid package.json file (in ${root}): not an object${convexCheckHint}`,
    });
  }

  if (
    typeof convexFileContents.dependencies !== "object" ||
    convexFileContents.dependencies === null ||
    !("convex" in convexFileContents.dependencies)
  ) {
    return await ctx.crash({
      printedMessage: `Can’t find the convex dependency in your package.json file (in ${root}).${convexCheckHint}`,
    });
  }

  if (typeof convexFileContents.dependencies.convex !== "string") {
    return await ctx.crash({
      printedMessage: `Invalid package.json file (in ${root}): \`dependencies.convex\` must be a string.${convexCheckHint}`,
    });
  }

  if (!semver.subset(convexFileContents.dependencies.convex, semverRange)) {
    return await ctx.crash({
      printedMessage: `The convex dependency in your package.json file (in ${root}) must be ${semverRange} (currently ${convexFileContents.dependencies.convex}).${convexCheckHint}`,
    });
  }
}
