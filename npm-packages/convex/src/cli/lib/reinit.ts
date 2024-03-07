import { Context, logFailure, showSpinner } from "../../bundler/context.js";
import {
  DeploymentType,
  fetchDeploymentCredentialsProvisioningDevOrProd,
} from "./api.js";
import { doCodegen } from "./codegen.js";
import {
  configName,
  readProjectConfig,
  upgradeOldAuthInfoToAuthConfig,
  writeProjectConfig,
} from "./config.js";
import { writeDeploymentEnvVar } from "./deployment.js";
import { finalizeConfiguration } from "./init.js";
import {
  functionsDir,
  validateOrSelectProject,
  validateOrSelectTeam,
} from "./utils.js";

export async function reinit(
  ctx: Context,
  deploymentType: DeploymentType = "prod",
  config: {
    team?: string | undefined;
    project?: string | undefined;
  },
) {
  const { teamSlug } = await validateOrSelectTeam(ctx, config.team, "Team:");

  const projectSlug = await validateOrSelectProject(
    ctx,
    config.project,
    teamSlug,
    "Configure project",
    "Project:",
  );
  if (!projectSlug) {
    logFailure(ctx, "Run the command again to create a new project instead.");
    await ctx.crash(1);
    return;
  }

  showSpinner(ctx, `Reinitializing project ${projectSlug}...\n`);

  const { deploymentName, url, adminKey } =
    await fetchDeploymentCredentialsProvisioningDevOrProd(
      ctx,
      { teamSlug, projectSlug },
      deploymentType,
    );

  const { configPath, projectConfig: existingProjectConfig } =
    await readProjectConfig(ctx);

  const functionsPath = functionsDir(configName(), existingProjectConfig);

  const { wroteToGitIgnore } = await writeDeploymentEnvVar(
    ctx,
    deploymentType,
    {
      team: teamSlug,
      project: projectSlug,
      deploymentName: deploymentName!,
    },
  );

  const projectConfig = await upgradeOldAuthInfoToAuthConfig(
    ctx,
    existingProjectConfig,
    functionsPath,
  );
  await writeProjectConfig(ctx, projectConfig, {
    deleteIfAllDefault: true,
  });

  await doCodegen({
    ctx,
    functionsDirectoryPath: functionsDir(configPath, projectConfig),
    typeCheckMode: "disable",
    quiet: true,
  });

  await finalizeConfiguration(
    ctx,
    functionsDir(configPath, projectConfig),
    deploymentType,
    url,
    wroteToGitIgnore,
  );

  return { deploymentName, url, adminKey };
}
