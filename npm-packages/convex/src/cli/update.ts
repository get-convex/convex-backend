import chalk from "chalk";
import { Command } from "@commander-js/extra-typings";
import { logMessage, oneoffContext } from "../bundler/context.js";
import { loadPackageJson } from "./lib/utils/utils.js";

export const update = new Command("update")
  .description("Print instructions for updating the convex package")
  .allowExcessArguments(false)
  .action(async () => {
    const ctx = oneoffContext();
    let updateInstructions = "npm install convex@latest\n";
    const packages = await loadPackageJson(ctx);
    const oldPackageNames = Object.keys(packages).filter((name) =>
      name.startsWith("@convex-dev"),
    );
    for (const pkg of oldPackageNames) {
      updateInstructions += `npm uninstall ${pkg}\n`;
    }

    logMessage(
      ctx,
      chalk.green(
        `To view the Convex changelog, go to https://news.convex.dev/tag/releases/\nWhen you are ready to upgrade, run the following commands:\n${updateInstructions}`,
      ),
    );
  });
