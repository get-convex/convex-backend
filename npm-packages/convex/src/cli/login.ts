import { Command, Option } from "@commander-js/extra-typings";
import { Context, oneoffContext } from "../bundler/context.js";
import { logFailure, logFinishedStep, logMessage } from "../bundler/log.js";
import { checkAuthorization, performLogin } from "./lib/login.js";
import {
  loadProjectLocalConfig,
  loadUuidForAnonymousUser,
} from "./lib/localDeployment/filePaths.js";
import {
  handleLinkToProject,
  listLegacyAnonymousDeployments,
} from "./lib/localDeployment/anonymous.js";
import {
  DASHBOARD_HOST,
  deploymentDashboardUrlPage,
  teamDashboardUrl,
} from "./lib/dashboard.js";
import { promptSearch, promptYesNo } from "./lib/utils/prompts.js";
import { bigBrainAPI, validateOrSelectTeam } from "./lib/utils/utils.js";
import {
  selectProject,
  updateEnvAndConfigForDeploymentSelection,
} from "./configure.js";
import {
  getDeploymentSelection,
  shouldAllowAnonymousDevelopment,
} from "./lib/deploymentSelection.js";
import {
  isAnonymousDeployment,
  removeAnonymousPrefix,
} from "./lib/deployment.js";
import {
  readGlobalConfig,
  globalConfigPath,
} from "./lib/utils/globalConfig.js";
import { getTeamsForUser } from "./lib/api.js";

const loginStatus = new Command("status")
  .description("Check login status and list accessible teams")
  .allowExcessArguments(false)
  .action(async () => {
    const ctx = await oneoffContext({
      url: undefined,
      adminKey: undefined,
      envFile: undefined,
    });

    const globalConfig = readGlobalConfig(ctx);
    const hasToken = globalConfig?.accessToken !== null;

    if (hasToken) {
      logMessage(`Convex account token found in: ${globalConfigPath()}`);
    } else {
      logMessage("No token found locally");
      return;
    }

    const isLoggedIn = await checkAuthorization(ctx, false);

    if (!isLoggedIn) {
      logMessage("Status: Not logged in");
      return;
    }

    logMessage("Status: Logged in");
    const teams = await getTeamsForUser(ctx);
    logMessage(
      `Teams: ${teams.length} team${teams.length === 1 ? "" : "s"} accessible`,
    );
    for (const team of teams) {
      logMessage(`  - ${team.name} (${team.slug})`);
    }
  });

export const login = new Command("login")
  .description("Login to Convex")
  .allowExcessArguments(false)
  .option(
    "--device-name <name>",
    "Provide a name for the device being authorized",
  )
  .option(
    "-f, --force",
    "Proceed with login even if a valid access token already exists for this device",
  )
  .option(
    "--no-open",
    "Don't automatically open the login link in the default browser",
  )
  .addOption(
    new Option(
      "--login-flow <mode>",
      `How to log in; defaults to guessing based on the environment.`,
    )
      .choices(["paste", "auto", "poll"] as const)
      .default("auto" as const),
  )
  .addOption(new Option("--link-deployments").hideHelp())
  // These options are hidden from the help/usage message, but allow overriding settings for testing.
  // Change the auth credentials with the auth provider
  .addOption(new Option("--override-auth-url <url>").hideHelp())
  .addOption(new Option("--override-auth-client <id>").hideHelp())
  .addOption(new Option("--override-auth-username <username>").hideHelp())
  .addOption(new Option("--override-auth-password <password>").hideHelp())
  // Skip the auth provider login and directly use this access token
  .addOption(new Option("--override-access-token <token>").hideHelp())
  // Automatically accept opt ins without prompting
  .addOption(new Option("--accept-opt-ins").hideHelp())
  // Dump the access token from the auth provider and skip authorization with Convex
  .addOption(new Option("--dump-access-token").hideHelp())
  // Hidden option for tests to check if the user is logged in.
  .addOption(new Option("--check-login").hideHelp())
  // Redirect to Vercel SSO integration URL
  .addOption(
    new Option(
      "--vercel",
      "Redirect to Vercel SSO integration for login",
    ).hideHelp(),
  )
  // Override the Vercel URL slug (defaults to 'convex')
  .addOption(new Option("--vercel-override <slug>").hideHelp())
  .addCommand(loginStatus)
  .addHelpCommand(false)
  .action(async (options, cmd: Command) => {
    const ctx = await oneoffContext({
      url: undefined,
      adminKey: undefined,
      envFile: undefined,
    });
    if (
      !options.force &&
      (await checkAuthorization(ctx, !!options.acceptOptIns))
    ) {
      logFinishedStep(
        "This device has previously been authorized and is ready for use with Convex.",
      );
      await handleLinkingDeployments(ctx, {
        interactive: !!options.linkDeployments,
      });
      return;
    }
    if (!options.force && options.checkLogin) {
      const isLoggedIn = await checkAuthorization(ctx, !!options.acceptOptIns);
      if (!isLoggedIn) {
        return ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          errForSentry: "You are not logged in.",
          printedMessage: "You are not logged in.",
        });
      }
    }
    if (!!options.overrideAuthUsername !== !!options.overrideAuthPassword) {
      cmd.error(
        "If overriding credentials, both username and password must be provided",
      );
    }

    const uuid = loadUuidForAnonymousUser(ctx);
    await performLogin(ctx, {
      ...options,
      anonymousId: uuid,
      vercel: options.vercel,
      vercelOverride: options.vercelOverride,
    });

    await handleLinkingDeployments(ctx, {
      interactive: !!options.linkDeployments,
    });
  });

async function handleLinkingDeployments(
  ctx: Context,
  args: {
    interactive: boolean;
  },
) {
  if (!shouldAllowAnonymousDevelopment()) {
    return;
  }

  // Check for project-local anonymous deployment first - this takes priority
  const projectLocal = loadProjectLocalConfig(ctx);
  if (
    projectLocal !== null &&
    isAnonymousDeployment(projectLocal.deploymentName)
  ) {
    const shouldLink = await promptYesNo(ctx, {
      message: `Would you like to link your existing deployment to your account? ("${projectLocal.deploymentName}")`,
      default: true,
    });
    if (!shouldLink) {
      logMessage(
        "Not linking your existing deployment. If you want to link it later, run `npx convex login --link-deployments`.",
      );
      logMessage(
        `Visit ${DASHBOARD_HOST} or run \`npx convex dev\` to get started with your new account.`,
      );
      return;
    }

    const { dashboardUrl } = await linkSingleDeployment(
      ctx,
      projectLocal.deploymentName,
      projectLocal.deploymentName,
    );
    logFinishedStep(`Visit ${dashboardUrl} to get started.`);
    return;
  }

  // No project-local deployment - check for legacy deployments
  const legacyDeployments = listLegacyAnonymousDeployments(ctx);
  if (legacyDeployments.length === 0) {
    if (args.interactive) {
      logMessage(
        "It doesn't look like you have any deployments to link. You can run `npx convex dev` to set up a new project or select an existing one.",
      );
    }
    return;
  }

  // Get the currently configured deployment (if any) for env var updates
  const deploymentSelection = await getDeploymentSelection(ctx, {
    url: undefined,
    adminKey: undefined,
    envFile: undefined,
  });
  const configuredDeployment =
    deploymentSelection.kind === "anonymous"
      ? deploymentSelection.deploymentName
      : null;

  if (!args.interactive) {
    // Non-interactive: link all legacy deployments automatically
    const message = getMessage(legacyDeployments.map((d) => d.deploymentName));
    const createProjects = await promptYesNo(ctx, {
      message,
      default: true,
    });
    if (!createProjects) {
      logMessage(
        "Not linking your existing deployments. If you want to link them later, run `npx convex login --link-deployments`.",
      );
      logMessage(
        `Visit ${DASHBOARD_HOST} or run \`npx convex dev\` to get started with your new account.`,
      );
      return;
    }

    const {
      team: { slug: teamSlug },
    } = await validateOrSelectTeam(
      ctx,
      undefined,
      "Choose a team for your deployments:",
    );
    const projectsRemaining = await getProjectsRemaining(ctx, teamSlug);
    if (legacyDeployments.length > projectsRemaining) {
      logFailure(
        `You have ${legacyDeployments.length} deployments to link, but only have ${projectsRemaining} projects remaining. If you'd like to choose which ones to link, run this command with the --link-deployments flag.`,
      );
      return;
    }

    let dashboardUrl = teamDashboardUrl(teamSlug);
    for (const deployment of legacyDeployments) {
      const result = await linkSingleDeployment(
        ctx,
        deployment.deploymentName,
        configuredDeployment,
        { teamSlug, projectSlug: null },
      );
      if (deployment.deploymentName === configuredDeployment) {
        dashboardUrl = result.dashboardUrl;
      }
    }
    logFinishedStep(
      `Successfully linked your deployments! Visit ${dashboardUrl} to get started.`,
    );
    return;
  }

  // Interactive mode: let user choose which legacy deployments to link
  while (true) {
    const currentLegacyDeployments = listLegacyAnonymousDeployments(ctx);
    if (currentLegacyDeployments.length === 0) {
      logMessage("All deployments have been linked.");
      break;
    }
    logMessage(
      getDeploymentListMessage(
        currentLegacyDeployments.map((d) => d.deploymentName),
      ),
    );
    const deploymentToLink = await promptSearch(ctx, {
      message: "Which deployment would you like to link to your account?",
      choices: currentLegacyDeployments.map((d) => ({
        name: d.deploymentName,
        value: d.deploymentName,
      })),
    });

    await linkSingleDeployment(ctx, deploymentToLink, configuredDeployment);

    const shouldContinue = await promptYesNo(ctx, {
      message: "Would you like to link another deployment?",
      default: true,
    });
    if (!shouldContinue) {
      break;
    }
  }
}

/**
 * Link a single deployment to a project, prompting for team and project selection.
 * Updates env vars if this is the currently configured deployment.
 */
async function linkSingleDeployment(
  ctx: Context,
  deploymentName: string,
  configuredDeployment: string | null,
  options?: {
    teamSlug?: string;
    projectSlug?: string | null;
  },
): Promise<{ dashboardUrl: string }> {
  const teamSlug =
    options?.teamSlug ??
    (
      await validateOrSelectTeam(
        ctx,
        undefined,
        "Choose a team for your deployment:",
      )
    ).team.slug;

  const projectSlug =
    options?.projectSlug ??
    (
      await selectProject(ctx, "ask", {
        team: teamSlug,
        devDeployment: "local",
        defaultProjectName: removeAnonymousPrefix(deploymentName),
      })
    ).projectSlug;

  const linkedDeployment = await handleLinkToProject(ctx, {
    deploymentName,
    teamSlug,
    projectSlug,
  });

  if (deploymentName === configuredDeployment) {
    await updateEnvAndConfigForDeploymentSelection(
      ctx,
      {
        url: linkedDeployment.deploymentUrl,
        deploymentName: linkedDeployment.deploymentName,
        teamSlug,
        projectSlug: linkedDeployment.projectSlug,
        deploymentType: "local",
      },
      configuredDeployment,
    );
  }

  return {
    dashboardUrl: deploymentDashboardUrlPage(
      linkedDeployment.deploymentName,
      "",
    ),
  };
}

async function getProjectsRemaining(ctx: Context, teamSlug: string) {
  const response = await bigBrainAPI<{ projectsRemaining: number }>({
    ctx,
    method: "GET",
    url: `teams/${teamSlug}/projects_remaining`,
  });

  return response.projectsRemaining;
}

function getDeploymentListMessage(anonymousDeploymentNames: string[]) {
  let message = `You have ${anonymousDeploymentNames.length} existing deployments.`;
  message += `\n\nDeployments:`;
  for (const deploymentName of anonymousDeploymentNames) {
    message += `\n- ${deploymentName}`;
  }
  return message;
}

function getMessage(anonymousDeploymentNames: string[]) {
  if (anonymousDeploymentNames.length === 1) {
    return `Would you like to link your existing deployment to your account? ("${anonymousDeploymentNames[0]}")`;
  }
  let message = `You have ${anonymousDeploymentNames.length} existing deployments. Would you like to link them to your account?`;
  message += `\n\nDeployments:`;
  for (const deploymentName of anonymousDeploymentNames) {
    message += `\n- ${deploymentName}`;
  }
  message += `\n\nYou can alternatively run \`npx convex login --link-deployments\` to interactively choose which deployments to add.`;
  return message;
}
