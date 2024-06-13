import { Command, Option } from "@commander-js/extra-typings";
import chalk from "chalk";
import { ensureHasConvexDependency } from "./lib/utils.js";
import { doInitCodegen, doCodegen } from "./lib/codegen.js";
import { logMessage, oneoffContext } from "../bundler/context.js";
import { getFunctionsDirectoryPath } from "./lib/config.js";

export const codegen = new Command("codegen")
  .summary("Generate backend type definitions")
  .description(
    "Generate types in `convex/_generated/` based on the current contents of `convex/`.",
  )
  .option(
    "--dry-run",
    "Print out the generated configuration to stdout instead of writing to convex directory",
  )
  .addOption(new Option("--debug").hideHelp())
  .addOption(
    new Option(
      "--typecheck <mode>",
      `Whether to check TypeScript files with \`tsc --noEmit\`.`,
    )
      .choices(["enable", "try", "disable"] as const)
      .default("try" as const),
  )
  .option(
    "--init",
    "Also (over-)write the default convex/README.md and convex/tsconfig.json files, otherwise only written when creating a new Convex project.",
  )
  // Experimental option
  .addOption(
    new Option(
      "--commonjs",
      "Generate CommonJS modules (CJS) instead of ECMAScript modules, the default. Bundlers typically take care of this conversion while bundling, so this setting is generally only useful for projects which do not use a bundler, typically Node.js projects. Convex functions can be written with either syntax.",
    ).hideHelp(),
  )
  .action(async (options) => {
    const ctx = oneoffContext;
    const functionsDirectoryPath = await getFunctionsDirectoryPath(ctx);

    // This also ensures the current directory is the project root.
    await ensureHasConvexDependency(ctx, "codegen");

    if (options.init) {
      await doInitCodegen(ctx, functionsDirectoryPath, false, {
        dryRun: options.dryRun,
        debug: options.debug,
      });
    }

    if (options.typecheck !== "disable") {
      logMessage(ctx, chalk.gray("Running TypeScript typecheck…"));
    }

    await doCodegen(ctx, functionsDirectoryPath, options.typecheck, {
      dryRun: options.dryRun,
      debug: options.debug,
      generateCommonJSApi: options.commonjs,
    });
  });
