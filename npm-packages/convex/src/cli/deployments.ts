import { Command } from "@commander-js/extra-typings";
import { readProjectConfig } from "./lib/config.js";
import chalk from "chalk";
import { bigBrainAPI } from "./lib/utils/utils.js";
import {
  logError,
  logMessage,
  logOutput,
  oneoffContext,
} from "../bundler/context.js";

type Deployment = {
  id: number;
  name: string;
  create_time: number;
  deployment_type: "dev" | "prod";
};

export const deployments = new Command("deployments")
  .description("List deployments associated with a project")
  .allowExcessArguments(false)
  .action(async () => {
    const ctx = await oneoffContext({
      url: undefined,
      adminKey: undefined,
      envFile: undefined,
    });
    const { projectConfig: config } = await readProjectConfig(ctx);

    const url = `teams/${config.team}/projects/${config.project}/deployments`;

    logMessage(ctx, `Deployments for project ${config.team}/${config.project}`);
    const deployments = (await bigBrainAPI({
      ctx,
      method: "GET",
      url,
    })) as Deployment[];
    logOutput(ctx, deployments);
    if (deployments.length === 0) {
      logError(ctx, chalk.yellow(`No deployments exist for project`));
    }
  });
