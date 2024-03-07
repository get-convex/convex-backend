import { Command } from "@commander-js/extra-typings";
import { logFinishedStep, oneoffContext } from "../bundler/context.js";
import { rootDirectory } from "./lib/utils.js";

export const logout = new Command("logout")
  .description("Log out of Convex on this machine")
  .action(async () => {
    const ctx = oneoffContext;

    const globalConfigDirectory = rootDirectory();
    ctx.fs.rm(globalConfigDirectory, { recursive: true });

    logFinishedStep(
      ctx,
      "You have been logged out of Convex.\n  Run `npx convex dev` to log in.",
    );
  });
