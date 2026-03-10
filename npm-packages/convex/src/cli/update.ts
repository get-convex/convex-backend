import { chalkStderr } from "chalk";
import { Command } from "@commander-js/extra-typings";
import { logMessage } from "../bundler/log.js";

export const update = new Command("update")
  .description("Print instructions for updating the convex package")
  .allowExcessArguments(false)
  .action(async () => {
    logMessage(
      chalkStderr.green(
        `To view the Convex changelog, go to https://news.convex.dev/tag/releases/\nWhen you are ready to upgrade, run the following command:\nnpm install convex@latest\n`,
      ),
    );
  });
