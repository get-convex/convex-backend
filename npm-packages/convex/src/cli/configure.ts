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
  getConfiguredCredentialsFromEnvVar,
  writeDeploymentEnvVar,
} from "./lib/deployment.js";
import { finalizeConfiguration } from "./lib/init.js";
import {
  bigBrainAPIMaybeThrows,
  functionsDir,
  getConfiguredDeployment,
  hasProjects,
  logAndHandleFetchError,
  selectDevDeploymentType,
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
import { readGlobalConfig } from "./lib/utils/globalConfig.js";

type DeploymentCredentials = {
  url: string;
  adminKey: string;
};

type ChosenConfiguration =
  // `--configure new`
  | "new"
  // `--configure existing`
  | "existing"
  // `--configure`
  | "ask"
  // `--configure` was not specified
  | null;

/**
 * As of writing, this is used by:
 * - `npx convex dev`
 * - `npx convex codegen`
 *
 * But is not used by `npx convex deploy` or other commands.
 */
export async function deploymentCredentialsOrConfigure(
  ctx: Context,
  chosenConfiguration: ChosenConfiguration,
  cmdOptions: {
    prod: boolean;
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
    devDeployment?: "cloud" | "local" | undefined;
    local?: boolean | undefined;
    cloud?: boolean | undefined;
    url?: string | undefined;
    adminKey?: string | undefined;
  },
  partitionId?: number | undefined,
): Promise<
  DeploymentCredentials & {
    deploymentName?: DeploymentName;
  }
> {
  const envVarCredentials = getConfiguredCredentialsFromEnvVar();
  const urlOverride = cmdOptions.url ?? envVarCredentials.url;
  const adminKeyOverride = cmdOptions.adminKey ?? envVarCredentials.adminKey;
  if (urlOverride !== undefined && adminKeyOverride !== undefined) {
    const credentials = await handleManuallySetUrlAndAdminKey(ctx, {
      url: urlOverride,
      adminKey: adminKeyOverride,
    });
    return { ...credentials };
  }

  const config = readGlobalConfig(ctx);
  const globallyForceCloud = !!config?.optOutOfLocalDevDeploymentsUntilBetaOver;
  if (globallyForceCloud && cmdOptions.local) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage:
        "Can't specify --local when local deployments are disabled on this machine. Run `npx convex disable-local-deployments --undo-global` to allow use of --local.",
    });
  }
  const { projectSlug, teamSlug, devDeployment } = await selectProject(
    ctx,
    chosenConfiguration,
    {
      team: cmdOptions.team,
      project: cmdOptions.project,
      devDeployment: cmdOptions.devDeployment,
      local: globallyForceCloud ? false : cmdOptions.local,
      cloud: globallyForceCloud ? true : cmdOptions.cloud,
      partitionId,
    },
  );

  // TODO complain about any non-default cmdOptions.localOptions here
  // because we're ignoring them if this isn't a local development.

  const deploymentOptions: DeploymentOptions = cmdOptions.prod
    ? { kind: "prod" }
    : devDeployment === "local"
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
    partitionId,
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

export async function handleManuallySetUrlAndAdminKey(
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
  chosenConfiguration: ChosenConfiguration,
  cmdOptions: {
    team?: string | undefined;
    project?: string | undefined;
    devDeployment?: "cloud" | "local" | undefined;
    local?: boolean | undefined;
    cloud?: boolean | undefined;
    partitionId?: number;
  },
): Promise<{
  teamSlug: string;
  projectSlug: string;
  devDeployment: "cloud" | "local";
}> {
  let result:
    | {
        teamSlug: string;
        projectSlug: string;
        devDeployment: "cloud" | "local";
      }
    | "AccessDenied"
    | null = null;

  const forceDevDeployment = cmdOptions.cloud
    ? "cloud"
    : cmdOptions.local
      ? "local"
      : undefined;

  if (chosenConfiguration === null) {
    result = await getConfiguredProjectSlugs(ctx);
    if (result !== null && result !== "AccessDenied") {
      return {
        ...result,
        ...(forceDevDeployment ? { devDeployment: forceDevDeployment } : {}),
      };
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
      return selectNewProject(ctx, chosenConfiguration, cmdOptions);
    case "existing":
      return selectExistingProject(ctx, chosenConfiguration, cmdOptions);
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
      devDeployment: "cloud" | "local";
    }
  | "AccessDenied"
  | null
> {
  // Try and infer the project from the deployment name
  const { name: deploymentName, type } = await getConfiguredDeployment(ctx);
  const devDeployment = type === "local" ? "local" : "cloud";
  if (deploymentName !== null) {
    const result = await getTeamAndProjectSlugForDeployment(ctx, {
      deploymentName,
    });
    if (result !== null) {
      return { ...result, devDeployment };
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
    return { teamSlug: team, projectSlug: project, devDeployment: "cloud" };
  }
  return null;
}

async function getTeamAndProjectSlugForDeployment(
  ctx: Context,
  selector: { deploymentName: string },
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
  chosenConfiguration: ChosenConfiguration,
  config: {
    team?: string | undefined;
    project?: string | undefined;
    devDeployment?: "cloud" | "local" | undefined;
    cloud?: boolean | undefined;
    local?: boolean | undefined;
    partitionId?: number | undefined;
  },
) {
  const { teamSlug: selectedTeam, chosen: didChooseBetweenTeams } =
    await validateOrSelectTeam(ctx, config.team, "Team:");
  let projectName: string = config.project || cwd;
  let choseProjectInteractively = false;
  if (!config.project) {
    projectName = await promptString(ctx, {
      message: "Project name:",
      default: cwd,
    });
    choseProjectInteractively = true;
  }

  const { devDeployment } = await selectDevDeploymentType(ctx, {
    chosenConfiguration,
    newOrExisting: "new",
    teamSlug: selectedTeam,
    userHasChosenSomethingInteractively:
      didChooseBetweenTeams || choseProjectInteractively,
    projectSlug: undefined,
    devDeploymentFromFlag: config.devDeployment,
    forceDevDeployment: config.local
      ? "local"
      : config.cloud
        ? "cloud"
        : undefined,
  });

  showSpinner(ctx, "Creating new Convex project...");

  let projectSlug, teamSlug, projectsRemaining;
  try {
    ({ projectSlug, teamSlug, projectsRemaining } = await createProject(ctx, {
      teamSlug: selectedTeam,
      projectName,
      partitionId: config.partitionId,
      // We have to create some deployment initially for a project.
      deploymentTypeToProvision: devDeployment === "local" ? "prod" : "dev",
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
  return { teamSlug, projectSlug, devDeployment };
}

async function selectExistingProject(
  ctx: Context,
  chosenConfiguration: ChosenConfiguration,
  config: {
    team?: string | undefined;
    project?: string | undefined;
    devDeployment?: "cloud" | "local" | undefined;
    local?: boolean | undefined;
    cloud?: boolean | undefined;
  },
): Promise<{
  teamSlug: string;
  projectSlug: string;
  devDeployment: "cloud" | "local";
}> {
  const { teamSlug, chosen } = await validateOrSelectTeam(
    ctx,
    config.team,
    "Team:",
  );

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
  const { devDeployment } = await selectDevDeploymentType(ctx, {
    chosenConfiguration,
    newOrExisting: "existing",
    teamSlug,
    projectSlug,
    userHasChosenSomethingInteractively: chosen || !config.project,
    devDeploymentFromFlag: config.devDeployment,
    forceDevDeployment: config.local
      ? "local"
      : config.cloud
        ? "cloud"
        : undefined,
  });

  showSpinner(ctx, `Reinitializing project ${projectSlug}...\n`);

  const { projectConfig: existingProjectConfig } = await readProjectConfig(ctx);

  const functionsPath = functionsDir(configName(), existingProjectConfig);

  await doCodegen(ctx, functionsPath, "disable");

  logFinishedStep(ctx, `Reinitialized project ${chalk.bold(projectSlug)}`);
  return { teamSlug, projectSlug, devDeployment };
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
    partitionId: number | undefined;
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
          options.partitionId,
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
