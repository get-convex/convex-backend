import chalk from "chalk";
import inquirer from "inquirer";
import {
  Context,
  logError,
  logFailure,
  logMessage,
} from "../bundler/context.js";
import {
  DeploymentType,
  DeploymentName,
  fetchDeploymentCredentialsForName,
  fetchDeploymentCredentialsProvisioningDevOrProdMaybeThrows,
} from "./lib/api.js";
import {
  ProjectConfig,
  enforceDeprecatedConfigField,
  readProjectConfig,
  upgradeOldAuthInfoToAuthConfig,
  writeProjectConfig,
} from "./lib/config.js";
import {
  eraseDeploymentEnvVar,
  writeDeploymentEnvVar,
} from "./lib/deployment.js";
import { init } from "./lib/init.js";
import { reinit } from "./lib/reinit.js";
import {
  ErrorData,
  functionsDir,
  getConfiguredDeploymentName,
  hasProject,
  hasProjects,
  hasTeam,
  logAndHandleFetchError,
  ThrowingFetchError,
} from "./lib/utils.js";
import { writeConvexUrlToEnvFile } from "./lib/envvars.js";

type DeploymentCredentials = {
  url: string;
  adminKey: string;
};

// This works like running `dev --once` for the first time
// but without a push.
// It only exists for backwards compatibility with existing
// scripts that used `convex init` or `convex reinit`.
export async function initOrReinitForDeprecatedCommands(
  ctx: Context,
  cmdOptions: {
    team?: string | undefined;
    project?: string | undefined;
    url?: string | undefined;
    adminKey?: string | undefined;
  },
) {
  const { url } = await deploymentCredentialsOrConfigure(ctx, null, {
    ...cmdOptions,
    prod: false,
  });
  // Try the CONVEX_URL write again in case the user had an existing
  // convex.json but didn't have CONVEX_URL in .env.local.
  const envVarWrite = await writeConvexUrlToEnvFile(ctx, url);
  if (envVarWrite !== null) {
    logMessage(
      ctx,
      chalk.green(
        `Saved the dev deployment URL as ${envVarWrite.envVar} to ${envVarWrite.envFile}`,
      ),
    );
  }
}

export async function deploymentCredentialsOrConfigure(
  ctx: Context,
  chosenConfiguration: "new" | "existing" | "ask" | null,
  cmdOptions: {
    prod: boolean;
    team?: string | undefined;
    project?: string | undefined;
    url?: string | undefined;
    adminKey?: string | undefined;
  },
): Promise<DeploymentCredentials & { deploymentName?: DeploymentName }> {
  const { url, adminKey } = cmdOptions;
  if (url !== undefined && adminKey !== undefined) {
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
  const deploymentType = cmdOptions.prod ? "prod" : "dev";
  const configuredDeployment =
    chosenConfiguration === null
      ? await getConfiguredDeploymentOrUpgrade(ctx, deploymentType)
      : null;
  // No configured deployment NOR existing config
  if (configuredDeployment === null) {
    const choice =
      chosenConfiguration !== "ask" && chosenConfiguration !== null
        ? chosenConfiguration
        : await askToConfigure(ctx);
    return await initOrReinit(ctx, choice, deploymentType, cmdOptions);
  }
  // Existing config but user doesn't have access to it
  if ("error" in configuredDeployment) {
    const projectConfig = (await readProjectConfig(ctx)).projectConfig;
    const choice = await askToReconfigure(
      ctx,
      projectConfig,
      configuredDeployment.error,
    );
    return initOrReinit(ctx, choice, deploymentType, cmdOptions);
  }
  const { deploymentName } = configuredDeployment;
  const adminKeyAndUrlForConfiguredDeployment =
    await fetchDeploymentCredentialsForName(
      ctx,
      deploymentName,
      deploymentType,
    );
  // Configured deployment and user has access
  if (!("error" in adminKeyAndUrlForConfiguredDeployment)) {
    return adminKeyAndUrlForConfiguredDeployment;
  }
  await checkForDeploymentTypeError(
    ctx,
    adminKeyAndUrlForConfiguredDeployment.error,
    deploymentType,
  );

  // Configured deployment and user doesn't has access to it
  const choice = await askToReconfigureNew(ctx, deploymentName);
  return initOrReinit(ctx, choice, deploymentType, cmdOptions);
}

async function checkForDeploymentTypeError(
  ctx: Context,
  error: unknown,
  deploymentType: DeploymentType,
) {
  let data: ErrorData | null = null;
  if (error instanceof ThrowingFetchError) {
    data = error.serverErrorData || null;
  }
  if (data && "code" in data && data.code === "DeploymentTypeMismatch") {
    if (deploymentType === "prod") {
      logFailure(
        ctx,
        "Use `npx convex deploy` to push changes to your production deployment",
      );
    } else {
      logFailure(
        ctx,
        "CONVEX_DEPLOYMENT is a production deployment, but --prod flag was not specified. " +
          "Use `npx convex dev --prod` to develop against this production deployment.",
      );
    }
    logError(ctx, chalk.red(data.message));
    await ctx.crash(1, "invalid filesystem data", error);
  }
}

async function getConfiguredDeploymentOrUpgrade(
  ctx: Context,
  deploymentType: DeploymentType,
) {
  const deploymentName = await getConfiguredDeploymentName(ctx);
  if (deploymentName !== null) {
    return { deploymentName };
  }
  return await upgradeOldConfigToDeploymentVar(ctx, deploymentType);
}

async function initOrReinit(
  ctx: Context,
  choice: "new" | "existing",
  deploymentType: DeploymentType,
  cmdOptions: { team?: string | undefined; project?: string | undefined },
): Promise<DeploymentCredentials> {
  switch (choice) {
    case "new":
      return (await init(ctx, deploymentType, cmdOptions))!;
    case "existing": {
      return (await reinit(ctx, deploymentType, cmdOptions))!;
    }
    default: {
      return choice;
    }
  }
}

async function upgradeOldConfigToDeploymentVar(
  ctx: Context,
  deploymentType: DeploymentType,
): Promise<{ deploymentName: string } | { error: unknown } | null> {
  const { configPath, projectConfig } = await readProjectConfig(ctx);
  const { team, project } = projectConfig;
  if (typeof team !== "string" || typeof project !== "string") {
    // The config is not a valid old config, proceed to init/reinit
    return null;
  }
  let devDeploymentName;
  try {
    const { deploymentName } =
      await fetchDeploymentCredentialsProvisioningDevOrProdMaybeThrows(
        ctx,
        { teamSlug: team, projectSlug: project },
        deploymentType,
      );
    devDeploymentName = deploymentName!;
  } catch (error) {
    // Could not retrieve the deployment name using the old config, proceed to reconfigure
    return { error };
  }
  await writeDeploymentEnvVar(ctx, deploymentType, {
    team,
    project,
    deploymentName: devDeploymentName,
  });
  logMessage(
    ctx,
    chalk.green(
      `Saved the ${deploymentType} deployment name as CONVEX_DEPLOYMENT to .env.local`,
    ),
  );
  const projectConfigWithoutAuthInfo = await upgradeOldAuthInfoToAuthConfig(
    ctx,
    projectConfig,
    functionsDir(configPath, projectConfig),
  );
  await writeProjectConfig(ctx, projectConfigWithoutAuthInfo, {
    deleteIfAllDefault: true,
  });
  return { deploymentName: devDeploymentName };
}

async function askToConfigure(ctx: Context): Promise<"new" | "existing"> {
  if (!(await hasProjects(ctx))) {
    return "new";
  }
  return await promptToInitWithProjects();
}

async function askToReconfigure(
  ctx: Context,
  projectConfig: ProjectConfig,
  error: unknown,
): Promise<"new" | "existing"> {
  const team = await enforceDeprecatedConfigField(ctx, projectConfig, "team");
  const project = await enforceDeprecatedConfigField(
    ctx,
    projectConfig,
    "project",
  );
  const [isExistingTeam, existingProject, hasAnyProjects] = await Promise.all([
    await hasTeam(ctx, team),
    await hasProject(ctx, team, project),
    await hasProjects(ctx),
  ]);

  // The config is good so there must be something else going on,
  // fatal with the original error
  if (isExistingTeam && existingProject) {
    return await logAndHandleFetchError(ctx, error);
  }

  if (isExistingTeam) {
    logFailure(
      ctx,
      `Project ${chalk.bold(project)} does not exist in your team ${chalk.bold(
        team,
      )}, as configured in ${chalk.bold("convex.json")}`,
    );
  } else {
    logFailure(
      ctx,
      `You don't have access to team ${chalk.bold(
        team,
      )}, as configured in ${chalk.bold("convex.json")}`,
    );
  }
  if (!hasAnyProjects) {
    const { confirmed } = await inquirer.prompt([
      {
        type: "confirm",
        name: "confirmed",
        message: `Create a new project?`,
        default: true,
      },
    ]);
    if (!confirmed) {
      logFailure(
        ctx,
        "Run `npx convex dev` in a directory with a valid convex.json.",
      );
      return await ctx.crash(1, "invalid filesystem data");
    }
    return "new";
  }

  return await promptToReconfigure();
}

async function askToReconfigureNew(
  ctx: Context,
  configuredDeploymentName: DeploymentName,
): Promise<"new" | "existing"> {
  logFailure(
    ctx,
    `You don't have access to the project with deployment ${chalk.bold(
      configuredDeploymentName,
    )}, as configured in ${chalk.bold("CONVEX_DEPLOYMENT")}`,
  );

  const hasAnyProjects = await hasProjects(ctx);

  if (!hasAnyProjects) {
    const { confirmed } = await inquirer.prompt([
      {
        type: "confirm",
        name: "confirmed",
        message: `Configure a new project?`,
        default: true,
      },
    ]);
    if (!confirmed) {
      logFailure(
        ctx,
        "Run `npx convex dev` in a directory with a valid CONVEX_DEPLOYMENT set",
      );
      return await ctx.crash(1, "invalid filesystem data");
    }
    return "new";
  }

  return await promptToReconfigure();
}

export async function promptToInitWithProjects(): Promise<"new" | "existing"> {
  const { choice } = await inquirer.prompt([
    {
      type: "list",
      name: "choice",
      message: `What would you like to configure?`,
      default: "new",
      choices: [
        { name: "a new project", value: "new" },
        { name: "an existing project", value: "existing" },
      ],
    },
  ]);
  return choice;
}

export async function promptToReconfigure(): Promise<"new" | "existing"> {
  const { choice } = await inquirer.prompt([
    {
      type: "list",
      name: "choice",
      message: `Configure a different project?`,
      default: "new",
      choices: [
        { name: "create new project", value: "new" },
        { name: "choose an existing project", value: "existing" },
      ],
    },
  ]);
  return choice;
}
