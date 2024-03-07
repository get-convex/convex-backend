import { Command, Option } from "@commander-js/extra-typings";
import path from "path";
import { oneoffContext } from "../bundler/context.js";
import { checkAuthorization, performLogin } from "./lib/login.js";
import { initOrReinitForDeprecatedCommands } from "./configure.js";

const cwd = path.basename(process.cwd());

// Initialize a new Convex project.
// This command is deprecated and hidden from the command help.
// `npx convex dev --once --configure=new` replaces it.
export const init = new Command("init")
  .description("Initialize a new Convex project in the current directory")
  .addOption(
    new Option(
      "--project <name>",
      `Name of the project to create. Defaults to \`${cwd}\` (the current directory)`,
    ),
  )
  .addOption(
    new Option(
      "--team <slug>",
      "Slug identifier for the team this project will belong to.",
    ),
  )
  .action(async (options) => {
    const ctx = oneoffContext;

    if (!(await checkAuthorization(ctx, false))) {
      await performLogin(ctx);
    }

    await initOrReinitForDeprecatedCommands(ctx, options);
  });
