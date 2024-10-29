import chalk from "chalk";
import {
  Context,
  logFailure,
  logFinishedStep,
  logMessage,
  logWarning,
  showSpinner,
} from "../bundler/context.js";
import {
  DeploymentType,
  DeploymentName,
  fetchDeploymentCredentialsProvisioningDevOrProdMaybeThrows,
  createProject,
} from "./lib/api.js";
import {
  configFilepath,
  configName,
  readProjectConfig,
  upgradeOldAuthInfoToAuthConfig,
  writeProjectConfig,
} from "./lib/config.js";
import {
  CONVEX_DEPLOYMENT_VAR_NAME,
  DeploymentDetails,
  eraseDeploymentEnvVar,
  writeDeploymentEnvVar,
} from "./lib/deployment.js";
import { finalizeConfiguration } from "./lib/init.js";
import {
  bigBrainAPIMaybeThrows,
  functionsDir,
  getConfiguredDeploymentName,
  hasProjects,
  logAndHandleFetchError,
  ThrowingFetchError,
  validateOrSelectProject,
  validateOrSelectTeam,
} from "./lib/utils/utils.js";
import { writeConvexUrlToEnvFile } from "./lib/envvars.js";
import path from "path";
import { projectDashboardUrl } from "./dashboard.js";
import { doCodegen, doInitCodegen } from "./lib/codegen.js";
import { handleLocalDeployment } from "./lib/localDeployment/localDeployment.js";
import { promptOptions, promptString } from "./lib/utils/prompts.js";

type DeploymentCredentials = {
  url: string;
  adminKey: string;
};

/**
 * As of writing, this is used by:
 * - `npx convex dev`
 * - `npx convex codegen`
 *
 * But is not used by `npx convex deploy` or other commands.
 */
export async function deploymentCredentialsOrConfigure(
  ctx: Context,
  chosenConfiguration: "new" | "existing" | "ask" | null,
  cmdOptions: {
    prod: boolean;
    local: boolean;
    localOptions: {
      ports?: {
        cloud: number;
        site: number;
      };
      backendVersion?: string | undefined;
      forceUpgrade: boolean;
    };
    team?: string | undefined;
    project?: string | undefined;
    url?: string | undefined;
    adminKey?: string | undefined;
  },
  partitionId?: number | undefined,
): Promise<
  DeploymentCredentials & {
    deploymentName?: DeploymentName;
  }
> {
  if (cmdOptions.url !== undefined && cmdOptions.adminKey !== undefined) {
    const credentials = await handleManuallySetUrlAndAdminKey(ctx, {
      url: cmdOptions.url,
      adminKey: cmdOptions.adminKey,
    });
    return { ...credentials };
  }
  const { projectSlug, teamSlug } = await selectProject(
    ctx,
    chosenConfiguration,
    {
      team: cmdOptions.team,
      project: cmdOptions.project,
      partitionId,
    },
  );
  const deploymentOptions: DeploymentOptions = cmdOptions.prod
    ? { kind: "prod" }
    : cmdOptions.local
      ? { kind: "local", ...cmdOptions.localOptions }
      : { kind: "dev" };
  const {
    deploymentName,
    deploymentUrl: url,
    adminKey,
  } = await ensureDeploymentProvisioned(ctx, {
    teamSlug,
    projectSlug,
    deploymentOptions,
  });
  await updateEnvAndConfigForDeploymentSelection(ctx, {
    url,
    deploymentName,
    teamSlug,
    projectSlug,
    deploymentType: deploymentOptions.kind,
  });

  return { deploymentName, url, adminKey };
}

async function handleManuallySetUrlAndAdminKey(
  ctx: Context,
  cmdOptions: { url: string; adminKey: string },
) {
  const { url, adminKey } = cmdOptions;
  const didErase = await eraseDeploymentEnvVar(ctx);
  if (didErase) {
    logMessage(
      ctx,
      chalk.yellowBright(
        `Removed the CONVEX_DEPLOYMENT environment variable from .env.local`,
      ),
    );
  }
  const envVarWrite = await writeConvexUrlToEnvFile(ctx, url);
  if (envVarWrite !== null) {
    logMessage(
      ctx,
      chalk.green(
        `Saved the given --url as ${envVarWrite.envVar} to ${envVarWrite.envFile}`,
      ),
    );
  }
  return { url, adminKey };
}

async function selectProject(
  ctx: Context,
  chosenConfiguration: "new" | "existing" | "ask" | null,
  cmdOptions: {
    team?: string | undefined;
    project?: string | undefined;
    partitionId?: number;
  },
): Promise<{ teamSlug: string; projectSlug: string }> {
  let result:
    | { teamSlug: string; projectSlug: string }
    | "AccessDenied"
    | null = null;
  if (chosenConfiguration === null) {
    result = await getConfiguredProjectSlugs(ctx);
    if (result !== null && result !== "AccessDenied") {
      return result;
    }
  }
  const reconfigure = result === "AccessDenied";
  // Prompt the user to select a project.
  const choice =
    chosenConfiguration !== "ask" && chosenConfiguration !== null
      ? chosenConfiguration
      : await askToConfigure(ctx, reconfigure);
  switch (choice) {
    case "new":
      return selectNewProject(ctx, cmdOptions);
    case "existing":
      return selectExistingProject(ctx, cmdOptions);
    default:
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: "No project selected.",
      });
  }
}

async function getConfiguredProjectSlugs(ctx: Context): Promise<
  | {
      projectSlug: string;
      teamSlug: string;
    }
  | "AccessDenied"
  | null
> {
  // Try and infer the project from the deployment name
  const deploymentName = await getConfiguredDeploymentName(ctx);
  if (deploymentName !== null) {
    const result = await getTeamAndProjectSlugForDeployment(ctx, {
      deploymentName,
      kind: "cloud",
    });
    if (result !== null) {
      return result;
    } else {
      logFailure(
        ctx,
        `You don't have access to the project with deployment ${chalk.bold(
          deploymentName,
        )}, as configured in ${chalk.bold(CONVEX_DEPLOYMENT_VAR_NAME)}`,
      );
      return "AccessDenied";
    }
  }
  // Try and infer the project from `convex.json`
  const { projectConfig } = await readProjectConfig(ctx);
  const { team, project } = projectConfig;
  if (typeof team === "string" && typeof project === "string") {
    const hasAccess = await hasAccessToProject(ctx, {
      teamSlug: team,
      projectSlug: project,
    });
    if (!hasAccess) {
      logFailure(
        ctx,
        `You don't have access to the project ${chalk.bold(project)} in team ${chalk.bold(team)} as configured in ${chalk.bold("convex.json")}`,
      );
      return "AccessDenied";
    }
    return { teamSlug: team, projectSlug: project };
  }
  return null;
}

async function getTeamAndProjectSlugForDeployment(
  ctx: Context,
  selector: { deploymentName: string; kind: "local" | "cloud" },
): Promise<{ teamSlug: string; projectSlug: string } | null> {
  try {
    const body = await bigBrainAPIMaybeThrows({
      ctx,
      url: `/api/deployment/${selector.deploymentName}/team_and_project`,
      method: "GET",
    });
    return { teamSlug: body.team, projectSlug: body.project };
  } catch (err) {
    if (
      err instanceof ThrowingFetchError &&
      (err.serverErrorData?.code === "DeploymentNotFound" ||
        err.serverErrorData?.code === "ProjectNotFound")
    ) {
      return null;
    }
    return logAndHandleFetchError(ctx, err);
  }
}

async function hasAccessToProject(
  ctx: Context,
  selector: { projectSlug: string; teamSlug: string },
): Promise<boolean> {
  try {
    await bigBrainAPIMaybeThrows({
      ctx,
      url: `/api/teams/${selector.teamSlug}/projects/${selector.projectSlug}/deployments`,
      method: "GET",
    });
    return true;
  } catch (err) {
    if (
      err instanceof ThrowingFetchError &&
      (err.serverErrorData?.code === "TeamNotFound" ||
        err.serverErrorData?.code === "ProjectNotFound")
    ) {
      return false;
    }
    return logAndHandleFetchError(ctx, err);
  }
}

const cwd = path.basename(process.cwd());
async function selectNewProject(
  ctx: Context,
  config: {
    team?: string | undefined;
    project?: string | undefined;
    partitionId?: number | undefined;
  },
) {
  const { teamSlug: selectedTeam, chosen: didChooseBetweenTeams } =
    await validateOrSelectTeam(ctx, config.team, "Team:");
  let projectName: string = config.project || cwd;
  if (!config.project) {
    projectName = await promptString(ctx, {
      message: "Project name:",
      default: cwd,
    });
  }

  showSpinner(ctx, "Creating new Convex project...");

  let projectSlug, teamSlug, projectsRemaining;
  try {
    ({ projectSlug, teamSlug, projectsRemaining } = await createProject(ctx, {
      teamSlug: selectedTeam,
      projectName,
      partitionId: config.partitionId,
    }));
  } catch (err) {
    logFailure(ctx, "Unable to create project.");
    return await logAndHandleFetchError(ctx, err);
  }
  const teamMessage = didChooseBetweenTeams
    ? " in team " + chalk.bold(teamSlug)
    : "";
  logFinishedStep(
    ctx,
    `Created project ${chalk.bold(
      projectSlug,
    )}${teamMessage}, manage it at ${chalk.bold(
      projectDashboardUrl(teamSlug, projectSlug),
    )}`,
  );

  if (projectsRemaining <= 2) {
    logWarning(
      ctx,
      chalk.yellow.bold(
        `Your account now has ${projectsRemaining} project${
          projectsRemaining === 1 ? "" : "s"
        } remaining.`,
      ),
    );
  }

  const { projectConfig: existingProjectConfig } = await readProjectConfig(ctx);
  const configPath = await configFilepath(ctx);
  const functionsPath = functionsDir(configPath, existingProjectConfig);
  await doInitCodegen(ctx, functionsPath, true);
  // Disable typechecking since there isn't any code yet.
  await doCodegen(ctx, functionsPath, "disable");
  return { teamSlug, projectSlug };
}

async function selectExistingProject(
  ctx: Context,
  config: {
    team?: string | undefined;
    project?: string | undefined;
  },
): Promise<{ teamSlug: string; projectSlug: string }> {
  const { teamSlug } = await validateOrSelectTeam(ctx, config.team, "Team:");

  const projectSlug = await validateOrSelectProject(
    ctx,
    config.project,
    teamSlug,
    "Configure project",
    "Project:",
  );
  if (projectSlug === null) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "Run the command again to create a new project instead.",
    });
  }

  showSpinner(ctx, `Reinitializing project ${projectSlug}...\n`);

  const { projectConfig: existingProjectConfig } = await readProjectConfig(ctx);

  const functionsPath = functionsDir(configName(), existingProjectConfig);

  await doCodegen(ctx, functionsPath, "disable");

  logFinishedStep(ctx, `Reinitialized project ${chalk.bold(projectSlug)}`);
  return { teamSlug, projectSlug };
}

async function askToConfigure(
  ctx: Context,
  reconfigure: boolean,
): Promise<"new" | "existing"> {
  if (!(await hasProjects(ctx))) {
    return "new";
  }
  return await promptOptions(ctx, {
    message: reconfigure
      ? "Configure a different project?"
      : "What would you like to configure?",
    default: "new",
    choices: [
      { name: "create a new project", value: "new" },
      { name: "choose an existing project", value: "existing" },
    ],
  });
}

type DeploymentOptions =
  | {
      kind: "prod";
    }
  | { kind: "dev" }
  | {
      kind: "local";
      ports?: {
        cloud: number;
        site: number;
      };
      backendVersion?: string;
      forceUpgrade: boolean;
    };

/**
 * This method assumes that the member has access to the selected project.
 */
async function ensureDeploymentProvisioned(
  ctx: Context,
  options: {
    teamSlug: string;
    projectSlug: string;
    deploymentOptions: DeploymentOptions;
  },
): Promise<DeploymentDetails> {
  switch (options.deploymentOptions.kind) {
    case "dev":
    case "prod": {
      const credentials =
        await fetchDeploymentCredentialsProvisioningDevOrProdMaybeThrows(
          ctx,
          { teamSlug: options.teamSlug, projectSlug: options.projectSlug },
          options.deploymentOptions.kind,
        );
      return {
        ...credentials,
        onActivity: null,
      };
    }
    case "local": {
      const credentials = await handleLocalDeployment(ctx, {
        teamSlug: options.teamSlug,
        projectSlug: options.projectSlug,
        ...options.deploymentOptions,
      });
      return credentials;
    }
    default:
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Invalid deployment type: ${(options.deploymentOptions as any).kind}`,
        errForSentry: `Invalid deployment type: ${(options.deploymentOptions as any).kind}`,
      });
  }
}

async function updateEnvAndConfigForDeploymentSelection(
  ctx: Context,
  options: {
    url: string;
    deploymentName: string;
    teamSlug: string;
    projectSlug: string;
    deploymentType: DeploymentType;
  },
) {
  const { configPath, projectConfig: existingProjectConfig } =
    await readProjectConfig(ctx);

  const functionsPath = functionsDir(configName(), existingProjectConfig);

  const { wroteToGitIgnore, changedDeploymentEnvVar } =
    await writeDeploymentEnvVar(ctx, options.deploymentType, {
      team: options.teamSlug,
      project: options.projectSlug,
      deploymentName: options.deploymentName,
    });
  const projectConfig = await upgradeOldAuthInfoToAuthConfig(
    ctx,
    existingProjectConfig,
    functionsPath,
  );
  await writeProjectConfig(ctx, projectConfig, {
    deleteIfAllDefault: true,
  });
  await finalizeConfiguration(ctx, {
    deploymentType: options.deploymentType,
    url: options.url,
    wroteToGitIgnore,
    changedDeploymentEnvVar,
    functionsPath: functionsDir(configPath, projectConfig),
  });
}
