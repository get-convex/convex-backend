import { Command } from "@commander-js/extra-typings";
import { logFinishedStep, oneoffContext } from "../bundler/context.js";
import { rootDirectory } from "./lib/utils.js";
import { recursivelyDelete } from "./lib/fsUtils.js";

export const logout = new Command("logout")
  .description("Log out of Convex on this machine")
  .action(async () => {
    const ctx = oneoffContext;

    const globalConfigDirectory = rootDirectory();
    recursivelyDelete(ctx, globalConfigDirectory);

    logFinishedStep(
      ctx,
      "You have been logged out of Convex.\n  Run `npx convex dev` to log in.",
    );
  });
