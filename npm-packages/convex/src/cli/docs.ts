import { Command } from "@commander-js/extra-typings";
import chalk from "chalk";
import open from "open";
import { oneoffContext } from "../bundler/context.js";
import { getTargetDeploymentName } from "./lib/deployment.js";
import { bigBrainFetch, deprecationCheckWarning } from "./lib/utils/utils.js";

export const docs = new Command("docs")
  .description("Open the docs in the browser")
  .option("--no-open", "Print docs URL instead of opening it in your browser")
  .action(async (options) => {
    const ctx = oneoffContext();
    // Usually we'd call `getConfiguredDeploymentName` but in this
    // command we don't care at all if the user is in the right directory
    const configuredDeployment = getTargetDeploymentName();
    const getCookieUrl = `get_cookie/${configuredDeployment}`;
    const fetch = await bigBrainFetch(ctx);
    try {
      const res = await fetch(getCookieUrl);
      deprecationCheckWarning(ctx, res);
      const { cookie } = await res.json();
      await openDocs(options.open, cookie);
    } catch {
      await openDocs(options.open);
    }
  });

async function openDocs(toOpen: boolean, cookie?: string) {
  let docsUrl = "https://docs.convex.dev";
  if (cookie !== undefined) {
    docsUrl += "/?t=" + cookie;
  }
  if (toOpen) {
    await open(docsUrl);
    console.log(chalk.green("Docs have launched! Check your browser."));
  } else {
    console.log(chalk.green(`Find Convex docs here: ${docsUrl}`));
  }
}
