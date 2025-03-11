import chalk from "chalk";
import { Context, logWarning } from "../../bundler/context.js";
import { teamDashboardUrl } from "../dashboard.js";
import { fetchTeamAndProject } from "./api.js";
import {
  bigBrainAPI,
  getAuthHeaderForBigBrain,
  getConfiguredDeployment,
  getConfiguredDeploymentNameOrCrash,
} from "./utils/utils.js";

async function warn(ctx: Context, title: string, subtitle: string) {
  const name = await getConfiguredDeploymentNameOrCrash(ctx);
  const { team } = await fetchTeamAndProject(ctx, name);

  logWarning(ctx, chalk.bold.yellow(title));
  logWarning(ctx, chalk.yellow(subtitle));
  logWarning(
    ctx,
    chalk.yellow(`Visit ${teamDashboardUrl(team)} to learn more.`),
  );
}

async function teamUsageState(ctx: Context) {
  const configuredDeployment = (await getConfiguredDeployment(ctx)).name;
  if (configuredDeployment === null) {
    return null;
  }

  const { teamId } = await fetchTeamAndProject(ctx, configuredDeployment);

  const { usageState } = (await bigBrainAPI({
    ctx,
    method: "GET",
    url: "dashboard/teams/" + teamId + "/usage/team_usage_state",
  })) as {
    usageState: "Default" | "Approaching" | "Exceeded" | "Disabled" | "Paused";
  };

  return usageState;
}

async function teamSpendingLimitsState(ctx: Context) {
  const configuredDeployment = (await getConfiguredDeployment(ctx)).name;
  if (configuredDeployment === null) {
    return null;
  }

  const { teamId } = await fetchTeamAndProject(ctx, configuredDeployment);

  const response = (await bigBrainAPI({
    ctx,
    method: "GET",
    url: "dashboard/teams/" + teamId + "/get_spending_limits",
  })) as {
    disableThresholdCents: number | null;
    state: null | "Running" | "Disabled" | "Warning";
  };

  return response.state;
}

export async function usageStateWarning(ctx: Context) {
  // Skip the warning if the user doesn’t have an auth token
  // (which can happen for instance when using a deploy key)
  const authHeader = await getAuthHeaderForBigBrain(ctx);
  if (authHeader === null) {
    return;
  }

  const [usageState, spendingLimitsState] = await Promise.all([
    teamUsageState(ctx),
    teamSpendingLimitsState(ctx),
  ]);

  if (spendingLimitsState === "Disabled") {
    await warn(
      ctx,
      "Your projects are disabled because you exceeded your spending limit.",
      "Increase it from the dashboard to re-enable your projects.",
    );
  } else if (usageState === "Approaching") {
    await warn(
      ctx,
      "Your projects are approaching the Starter plan limits.",
      "Consider upgrading to avoid service interruption.",
    );
  } else if (usageState === "Exceeded") {
    await warn(
      ctx,
      "Your projects are above the Starter plan limits.",
      "Decrease your usage or upgrade to avoid service interruption.",
    );
  } else if (usageState === "Disabled") {
    await warn(
      ctx,
      "Your projects are disabled because the team exceeded Starter plan limits.",
      "Decrease your usage or upgrade to re-enable your projects.",
    );
  } else if (usageState === "Paused") {
    await warn(
      ctx,
      "Your projects are disabled because the team previously exceeded Starter plan limits.",
      "Restore your projects by going to the dashboard.",
    );
  }
}
