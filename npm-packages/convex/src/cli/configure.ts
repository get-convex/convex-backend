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
  DeploymentSelectionWithinProject,
  loadSelectedDeploymentCredentials,
  checkAccessToSelectedProject,
  validateDeploymentSelectionForExistingDeployment,
} from "./lib/api.js";
import {
  configFilepath,
  configName,
  readProjectConfig,
  upgradeOldAuthInfoToAuthConfig,
  writeProjectConfig,
} from "./lib/config.js";
import {
  DeploymentDetails,
  eraseDeploymentEnvVar,
  writeDeploymentEnvVar,
} from "./lib/deployment.js";
import { finalizeConfiguration } from "./lib/init.js";
import {
  functionsDir,
  hasProjects,
  logAndHandleFetchError,
  selectDevDeploymentType,
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
import {
  DeploymentSelection,
  deploymentNameFromSelection,
} from "./lib/deploymentSelection.js";
import { ensureLoggedIn } from "./lib/login.js";
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

type ConfigureCmdOptions = {
  selectionWithinProject: DeploymentSelectionWithinProject;
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
  envFile?: string | undefined;
  overrideAuthUrl?: string | undefined;
  overrideAuthClient?: string | undefined;
  overrideAuthUsername?: string | undefined;
  overrideAuthPassword?: string | undefined;
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
  deploymentSelection: DeploymentSelection,
  chosenConfiguration: ChosenConfiguration,
  cmdOptions: ConfigureCmdOptions,
  partitionId?: number | undefined,
): Promise<
  DeploymentCredentials & {
    deploymentFields: {
      deploymentName: DeploymentName;
      deploymentType: string;
      projectSlug: string | null;
      teamSlug: string | null;
    } | null;
  }
> {
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

  switch (deploymentSelection.kind) {
    case "existingDeployment":
      await validateDeploymentSelectionForExistingDeployment(
        ctx,
        cmdOptions.selectionWithinProject,
        deploymentSelection.deploymentToActOn.source,
      );
      if (deploymentSelection.deploymentToActOn.deploymentFields === null) {
        // erase `CONVEX_DEPLOYMENT` from .env.local + set the url env var
        await handleManuallySetUrlAndAdminKey(ctx, {
          url: deploymentSelection.deploymentToActOn.url,
          adminKey: deploymentSelection.deploymentToActOn.adminKey,
        });
      }
      return {
        url: deploymentSelection.deploymentToActOn.url,
        adminKey: deploymentSelection.deploymentToActOn.adminKey,
        deploymentFields:
          deploymentSelection.deploymentToActOn.deploymentFields,
      };
    case "chooseProject": {
      await ensureLoggedIn(ctx, {
        overrideAuthUrl: cmdOptions.overrideAuthUrl,
        overrideAuthClient: cmdOptions.overrideAuthClient,
        overrideAuthUsername: cmdOptions.overrideAuthUsername,
        overrideAuthPassword: cmdOptions.overrideAuthPassword,
      });
      return await handleChooseProject(
        ctx,
        chosenConfiguration,
        {
          globallyForceCloud,
          partitionId,
        },
        cmdOptions,
      );
    }
    case "preview":
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: "Use `npx convex deploy` to use preview deployments.",
      });
    case "deploymentWithinProject": {
      await ensureLoggedIn(ctx, {
        overrideAuthUrl: cmdOptions.overrideAuthUrl,
        overrideAuthClient: cmdOptions.overrideAuthClient,
        overrideAuthUsername: cmdOptions.overrideAuthUsername,
        overrideAuthPassword: cmdOptions.overrideAuthPassword,
      });
      const accessResult = await checkAccessToSelectedProject(
        ctx,
        deploymentSelection.targetProject,
      );
      if (accessResult.kind === "noAccess") {
        logMessage(ctx, "You don't have access to the selected project.");
        const result = await handleChooseProject(
          ctx,
          chosenConfiguration,
          {
            globallyForceCloud,
            partitionId,
          },
          cmdOptions,
        );
        return result;
      }
      if (chosenConfiguration === "new") {
        const result = await handleChooseProject(
          ctx,
          chosenConfiguration,
          {
            globallyForceCloud,
            partitionId,
          },
          cmdOptions,
        );
        return result;
      }

      const selectedDeployment = await loadSelectedDeploymentCredentials(
        ctx,
        deploymentSelection,
        cmdOptions.selectionWithinProject,
        // We'll start running it below
        { ensureLocalRunning: false },
      );

      if (selectedDeployment.deploymentFields !== null) {
        await updateEnvAndConfigForDeploymentSelection(
          ctx,
          {
            url: selectedDeployment.url,
            deploymentName: selectedDeployment.deploymentFields.deploymentName,
            teamSlug: selectedDeployment.deploymentFields.teamSlug,
            projectSlug: selectedDeployment.deploymentFields.projectSlug,
            deploymentType: selectedDeployment.deploymentFields
              .deploymentType as DeploymentType,
          },
          deploymentNameFromSelection(deploymentSelection),
        );
        if (
          selectedDeployment.deploymentFields !== null &&
          selectedDeployment.deploymentFields.deploymentType === "local"
        ) {
          await handleLocalDeployment(ctx, {
            teamSlug: selectedDeployment.deploymentFields.teamSlug!,
            projectSlug: selectedDeployment.deploymentFields.projectSlug!,
            forceUpgrade: cmdOptions.localOptions.forceUpgrade,
            ports: cmdOptions.localOptions.ports,
            backendVersion: cmdOptions.localOptions.backendVersion,
          });
        }
        return selectedDeployment;
      }
      return {
        url: selectedDeployment.url,
        adminKey: selectedDeployment.adminKey,
        deploymentFields: selectedDeployment.deploymentFields,
      };
    }
  }
}

async function handleChooseProject(
  ctx: Context,
  chosenConfiguration: ChosenConfiguration,
  args: {
    globallyForceCloud: boolean;
    partitionId?: number | undefined;
  },
  cmdOptions: ConfigureCmdOptions,
): Promise<
  DeploymentCredentials & {
    deploymentFields: {
      deploymentName: DeploymentName;
      deploymentType: DeploymentType;
      projectSlug: string;
      teamSlug: string;
    };
  }
> {
  await ensureLoggedIn(ctx, {
    overrideAuthUrl: cmdOptions.overrideAuthUrl,
    overrideAuthClient: cmdOptions.overrideAuthClient,
    overrideAuthUsername: cmdOptions.overrideAuthUsername,
    overrideAuthPassword: cmdOptions.overrideAuthPassword,
  });
  const project = await selectProject(ctx, chosenConfiguration, {
    team: cmdOptions.team,
    project: cmdOptions.project,
    devDeployment: cmdOptions.devDeployment,
    local: args.globallyForceCloud ? false : cmdOptions.local,
    cloud: args.globallyForceCloud ? true : cmdOptions.cloud,
    partitionId: args.partitionId,
  });
  // TODO complain about any non-default cmdOptions.localOptions here
  // because we're ignoring them if this isn't a local development.

  const deploymentOptions: DeploymentOptions =
    cmdOptions.selectionWithinProject.kind === "prod"
      ? { kind: "prod" }
      : project.devDeployment === "local"
        ? { kind: "local", ...cmdOptions.localOptions }
        : { kind: "dev" };
  const {
    deploymentName,
    deploymentUrl: url,
    adminKey,
  } = await ensureDeploymentProvisioned(ctx, {
    teamSlug: project.teamSlug,
    projectSlug: project.projectSlug,
    deploymentOptions,
    partitionId: args.partitionId,
  });
  await updateEnvAndConfigForDeploymentSelection(
    ctx,
    {
      url,
      deploymentName,
      teamSlug: project.teamSlug,
      projectSlug: project.projectSlug,
      deploymentType: deploymentOptions.kind,
    },
    null,
  );
  return {
    url,
    adminKey,
    deploymentFields: {
      deploymentName,
      deploymentType: deploymentOptions.kind,
      projectSlug: project.projectSlug,
      teamSlug: project.teamSlug,
    },
  };
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
  // Prompt the user to select a project.
  const choice =
    chosenConfiguration !== "ask" && chosenConfiguration !== null
      ? chosenConfiguration
      : await askToConfigure(ctx);
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

async function askToConfigure(ctx: Context): Promise<"new" | "existing"> {
  if (!(await hasProjects(ctx))) {
    return "new";
  }
  return await promptOptions(ctx, {
    message: "What would you like to configure?",
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
          {
            kind: "teamAndProjectSlugs",
            teamSlug: options.teamSlug,
            projectSlug: options.projectSlug,
          },
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
    teamSlug: string | null;
    projectSlug: string | null;
    deploymentType: DeploymentType;
  },
  existingValue: string | null,
) {
  const { configPath, projectConfig: existingProjectConfig } =
    await readProjectConfig(ctx);

  const functionsPath = functionsDir(configName(), existingProjectConfig);

  const { wroteToGitIgnore, changedDeploymentEnvVar } =
    await writeDeploymentEnvVar(
      ctx,
      options.deploymentType,
      {
        team: options.teamSlug,
        project: options.projectSlug,
        deploymentName: options.deploymentName,
      },
      existingValue,
    );
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
