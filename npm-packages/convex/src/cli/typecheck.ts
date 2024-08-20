import chalk from "chalk";
import { functionsDir, ensureHasConvexDependency } from "./lib/utils.js";
import { Command } from "@commander-js/extra-typings";
import { readConfig } from "./lib/config.js";
import { typeCheckFunctions } from "./lib/typecheck.js";
import {
  logFinishedStep,
  logMessage,
  oneoffContext,
} from "../bundler/context.js";

// Experimental (it's going to fail sometimes) TypeScript type checking.
// Includes a separate command to help users debug their TypeScript configs.

export type TypecheckResult = "cantTypeCheck" | "success" | "typecheckFailed";

/** Run the TypeScript compiler, as configured during  */
export const typecheck = new Command("typecheck")
  .description(
    "Run TypeScript typechecking on your Convex functions with `tsc --noEmit`.",
  )
  .action(async () => {
    const ctx = oneoffContext;
    const { configPath, config: localConfig } = await readConfig(ctx, false);
    await ensureHasConvexDependency(ctx, "typecheck");
    await typeCheckFunctions(
      ctx,
      functionsDir(configPath, localConfig.projectConfig),
      async (typecheckResult, logSpecificError) => {
        logSpecificError?.();
        if (typecheckResult === "typecheckFailed") {
          logMessage(ctx, chalk.gray("Typecheck failed"));
          return await ctx.crash({
            exitCode: 1,
            errorType: "invalid filesystem data",
            printedMessage: null,
          });
        } else if (typecheckResult === "cantTypeCheck") {
          logMessage(
            ctx,
            chalk.gray("Unable to typecheck; is TypeScript installed?"),
          );
          return await ctx.crash({
            exitCode: 1,
            errorType: "invalid filesystem data",
            printedMessage: null,
          });
        } else {
          logFinishedStep(
            ctx,
            "Typecheck passed: `tsc --noEmit` completed with exit code 0.",
          );
          return await ctx.flushAndExit(0);
        }
      },
    );
  });
