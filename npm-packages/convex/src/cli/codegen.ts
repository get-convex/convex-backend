import { Command, Option } from "@commander-js/extra-typings";
import { oneoffContext } from "../bundler/context.js";
import { runCodegen } from "./lib/components.js";

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
  .addOption(new Option("--admin-key <adminKey>").hideHelp())
  .addOption(new Option("--url <url>").hideHelp())
  .addOption(new Option("--live-component-sources").hideHelp())
  // Experimental option
  .addOption(
    new Option(
      "--commonjs",
      "Generate CommonJS modules (CJS) instead of ECMAScript modules, the default. Bundlers typically take care of this conversion while bundling, so this setting is generally only useful for projects which do not use a bundler, typically Node.js projects. Convex functions can be written with either syntax.",
    ).hideHelp(),
  )
  .action(async (options) => {
    const ctx = oneoffContext;

    await runCodegen(ctx, {
      dryRun: !!options.dryRun,
      debug: !!options.debug,
      typecheck: options.typecheck,
      init: !!options.init,
      commonjs: !!options.commonjs,
      url: options.url,
      adminKey: options.adminKey,
      liveComponentSources: !!options.liveComponentSources,
    });
  });
