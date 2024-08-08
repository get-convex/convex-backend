import chalk from "chalk";
import {
  Context,
  logError,
  logFailure,
  logVerbose,
} from "../../bundler/context.js";
import {
  deploymentNameFromAdminKeyOrCrash,
  deploymentTypeFromAdminKey,
  getConfiguredDeploymentFromEnvVar,
  getTeamAndProjectFromPreviewAdminKey,
  isPreviewDeployKey,
} from "./deployment.js";
import { buildEnvironment } from "./envvars.js";
import { checkAuthorization, performLogin } from "./login.js";
import {
  CONVEX_DEPLOY_KEY_ENV_VAR_NAME,
  bigBrainAPI,
  bigBrainAPIMaybeThrows,
  getAuthHeaderForBigBrain,
  getConfiguredDeploymentName,
  getConfiguredDeploymentOrCrash,
  readAdminKeyFromEnvVar,
} from "./utils.js";

export type DeploymentName = string;
export type DeploymentType = "dev" | "prod" | "local";

export type Project = {
  id: string;
  name: string;
  slug: string;
  isDemo: boolean;
};

type AdminKey = string;

// Provision a new empty project and return the slugs.
export async function createProject(
  ctx: Context,
  {
    teamSlug: selectedTeamSlug,
    projectName,
  }: { teamSlug: string; projectName: string },
): Promise<{
  projectSlug: string;
  teamSlug: string;
  projectsRemaining: number;
}> {
  const provisioningArgs = {
    team: selectedTeamSlug,
    projectName,
    // TODO: Consider allowing projects with no deployments, or consider switching
    // to provisioning prod on creation.
    deploymentType: "dev",
    backendVersionOverride: process.env.CONVEX_BACKEND_VERSION_OVERRIDE,
  };
  const data = await bigBrainAPI({
    ctx,
    method: "POST",
    url: "create_project",
    data: provisioningArgs,
  });
  const { projectSlug, teamSlug, projectsRemaining } = data;
  if (
    projectSlug === undefined ||
    teamSlug === undefined ||
    projectsRemaining === undefined
  ) {
    const error =
      "Unexpected response during provisioning: " + JSON.stringify(data);
    logError(ctx, chalk.red(error));
    return await ctx.crash(1, "transient", error);
  }
  return {
    projectSlug,
    teamSlug,
    projectsRemaining,
  };
}

// Init
// Provision a new empty project and return the new deployment credentials.
export async function createProjectProvisioningDevOrProd(
  ctx: Context,
  {
    teamSlug: selectedTeamSlug,
    projectName,
  }: { teamSlug: string; projectName: string },
  firstDeploymentType: DeploymentType,
): Promise<{
  projectSlug: string;
  teamSlug: string;
  deploymentName: string;
  url: string;
  adminKey: AdminKey;
  projectsRemaining: number;
}> {
  const provisioningArgs = {
    team: selectedTeamSlug,
    projectName,
    deploymentType: firstDeploymentType,
    backendVersionOverride: process.env.CONVEX_BACKEND_VERSION_OVERRIDE,
  };
  const data = await bigBrainAPI({
    ctx,
    method: "POST",
    url: "create_project",
    data: provisioningArgs,
  });
  const {
    projectSlug,
    teamSlug,
    deploymentName,
    adminKey,
    projectsRemaining,
    prodUrl: url,
  } = data;
  if (
    projectSlug === undefined ||
    teamSlug === undefined ||
    deploymentName === undefined ||
    url === undefined ||
    adminKey === undefined ||
    projectsRemaining === undefined
  ) {
    const error =
      "Unexpected response during provisioning: " + JSON.stringify(data);
    logError(ctx, chalk.red(error));
    return await ctx.crash(1, "transient", error);
  }
  return {
    projectSlug,
    teamSlug,
    deploymentName,
    url,
    adminKey,
    projectsRemaining,
  };
}

// Dev
export async function fetchDeploymentCredentialsForName(
  ctx: Context,
  deploymentName: DeploymentName,
  deploymentType: DeploymentType,
): Promise<
  | {
      deploymentName: string;
      adminKey: string;
      url: string;
      deploymentType: DeploymentType;
    }
  | { error: unknown }
> {
  let data;
  try {
    data = await bigBrainAPIMaybeThrows({
      ctx,
      method: "POST",
      url: "deployment/authorize_for_name",
      data: {
        deploymentName,
        deploymentType,
      },
    });
  } catch (error: unknown) {
    return { error };
  }
  const adminKey: string = data.adminKey;
  const url: string = data.url;
  const resultDeploymentType: DeploymentType = data.deploymentType;
  if (adminKey === undefined || url === undefined) {
    const msg = "Unknown error during authorization: " + JSON.stringify(data);
    logError(ctx, chalk.red(msg));
    return await ctx.crash(1, "transient", new Error(msg));
  }
  return {
    deploymentName,
    adminKey,
    url,
    deploymentType: resultDeploymentType,
  };
}

export type DeploymentSelection =
  | { kind: "deployKey" }
  | { kind: "previewName"; previewName: string }
  | { kind: "deploymentName"; deploymentName: string }
  | { kind: "ownProd" }
  | { kind: "ownDev" }
  | { kind: "urlWithAdminKey"; url: string; adminKey: string }
  | { kind: "urlWithLogin"; url: string };

export function storeAdminKeyEnvVar(adminKeyOption?: string | null) {
  if (adminKeyOption) {
    // So we don't have to worry about passing through the admin key everywhere
    // if it's explicitly overridden by a CLI option, override the env variable
    // directly.
    process.env[CONVEX_DEPLOY_KEY_ENV_VAR_NAME] = adminKeyOption;
  }
}

export type DeploymentSelectionOptions = {
  // Whether to default to prod
  prod?: boolean | undefined;

  previewName?: string | undefined;
  deploymentName?: string | undefined;
  url?: string | undefined;
  adminKey?: string | undefined;
};

export function deploymentSelectionFromOptions(
  options: DeploymentSelectionOptions,
): DeploymentSelection {
  storeAdminKeyEnvVar(options.adminKey);
  const adminKey = readAdminKeyFromEnvVar();
  if (options.url !== undefined) {
    if (adminKey) {
      return { kind: "urlWithAdminKey", url: options.url, adminKey };
    }
    return { kind: "urlWithLogin", url: options.url };
  }
  if (options.previewName !== undefined) {
    return { kind: "previewName", previewName: options.previewName };
  }
  if (options.deploymentName !== undefined) {
    return { kind: "deploymentName", deploymentName: options.deploymentName };
  }
  if (adminKey !== undefined) {
    return { kind: "deployKey" };
  }
  return { kind: options.prod === true ? "ownProd" : "ownDev" };
}

// Deploy
export async function fetchDeploymentCredentialsWithinCurrentProject(
  ctx: Context,
  deploymentSelection: DeploymentSelection,
): Promise<{
  url: string;
  adminKey: AdminKey;
  deploymentName?: string;
  deploymentType?: string | undefined;
}> {
  if (deploymentSelection.kind === "urlWithAdminKey") {
    return {
      adminKey: deploymentSelection.adminKey,
      url: deploymentSelection.url,
    };
  }

  const configuredAdminKey = readAdminKeyFromEnvVar();

  // Crash if we know that DEPLOY_KEY (adminKey) is required
  if (configuredAdminKey === undefined) {
    const buildEnvironmentExpectsConvexDeployKey = buildEnvironment();
    if (buildEnvironmentExpectsConvexDeployKey) {
      logFailure(
        ctx,
        `${buildEnvironmentExpectsConvexDeployKey} build environment detected but ${CONVEX_DEPLOY_KEY_ENV_VAR_NAME} is not set. ` +
          `Set this environment variable to deploy from this environment. See https://docs.convex.dev/production/hosting`,
      );
      await ctx.crash(1);
    }
    const header = await getAuthHeaderForBigBrain(ctx);
    if (!header) {
      logFailure(
        ctx,
        `Error: You are not logged in. Log in with \`npx convex dev\` or set the ${CONVEX_DEPLOY_KEY_ENV_VAR_NAME} environment variable. ` +
          `See https://docs.convex.dev/production/hosting`,
      );
      await ctx.crash(1);
    }
    const configuredDeployment = await getConfiguredDeploymentName(ctx);
    if (configuredDeployment === null) {
      logFailure(
        ctx,
        "No CONVEX_DEPLOYMENT set, run `npx convex dev` to configure a Convex project",
      );
      await ctx.crash(1);
    }
  }

  const data = await fetchDeploymentCredentialsWithinCurrentProjectInner(
    ctx,
    deploymentSelection,
    configuredAdminKey,
  );
  const { deploymentName, adminKey, deploymentType, url } = data;
  if (
    adminKey === undefined ||
    url === undefined ||
    deploymentName === undefined
  ) {
    const msg = "Unknown error during authorization: " + JSON.stringify(data);
    logError(ctx, chalk.red(msg));
    return await ctx.crash(1, "transient", new Error(msg));
  }
  return {
    deploymentName,
    adminKey,
    url,
    deploymentType,
  };
}

type ProjectSelection =
  | {
      kind: "deploymentName";
      // Identify a project by one of the deployments in it.
      deploymentName: string;
    }
  | {
      kind: "teamAndProjectSlugs";
      // Identify a project by its team and slug.
      teamSlug: string;
      projectSlug: string;
    };

export async function projectSelection(
  ctx: Context,
  configuredDeployment: string | null,
  configuredAdminKey: string | undefined,
): Promise<ProjectSelection> {
  if (
    configuredAdminKey !== undefined &&
    isPreviewDeployKey(configuredAdminKey)
  ) {
    const { teamSlug, projectSlug } =
      await getTeamAndProjectFromPreviewAdminKey(ctx, configuredAdminKey);
    return {
      kind: "teamAndProjectSlugs",
      teamSlug,
      projectSlug,
    };
  }
  if (configuredAdminKey !== undefined) {
    return {
      kind: "deploymentName",
      deploymentName: await deploymentNameFromAdminKeyOrCrash(
        ctx,
        configuredAdminKey,
      ),
    };
  }
  if (configuredDeployment) {
    return {
      kind: "deploymentName",
      deploymentName: configuredDeployment,
    };
  }
  logFailure(
    ctx,
    "Select project by setting `CONVEX_DEPLOYMENT` with `npx convex dev` or `CONVEX_DEPLOY_KEY` from the Convex dashboard.",
  );
  return await ctx.crash(1);
}

async function fetchDeploymentCredentialsWithinCurrentProjectInner(
  ctx: Context,
  deploymentSelection: Exclude<
    DeploymentSelection,
    { kind: "urlWithAdminKey"; url: string; adminKey: string }
  >,
  configuredAdminKey: string | undefined,
): Promise<{
  deploymentName?: string;
  adminKey?: string;
  url?: string;
  deploymentType?: string;
}> {
  const configuredDeployment = getConfiguredDeploymentFromEnvVar().name;
  switch (deploymentSelection.kind) {
    case "ownDev": {
      return {
        ...(await fetchExistingDevDeploymentCredentialsOrCrash(
          ctx,
          configuredDeployment!,
        )),
        deploymentName: configuredDeployment!,
      };
    }
    case "ownProd":
      return await bigBrainAPI({
        ctx,
        method: "POST",
        url: "deployment/authorize_prod",
        data: {
          deploymentName: configuredDeployment,
        },
      });
    case "previewName":
      return await bigBrainAPI({
        ctx,
        method: "POST",
        url: "deployment/authorize_preview",
        data: {
          previewName: deploymentSelection.previewName,
          projectSelection: await projectSelection(
            ctx,
            configuredDeployment,
            configuredAdminKey,
          ),
        },
      });
    case "deploymentName":
      return await bigBrainAPI({
        ctx,
        method: "POST",
        url: "deployment/authorize_within_current_project",
        data: {
          selectedDeploymentName: deploymentSelection.deploymentName,
          projectSelection: await projectSelection(
            ctx,
            configuredDeployment,
            configuredAdminKey,
          ),
        },
      });
    case "deployKey": {
      const deploymentName = await deploymentNameFromAdminKeyOrCrash(
        ctx,
        configuredAdminKey!,
      );
      let url = await deriveUrlFromAdminKey(ctx, configuredAdminKey!);
      // We cannot derive the deployment URL from the deploy key
      // when running against local big brain, so use the name to get the URL.
      if (process.env.CONVEX_PROVISION_HOST !== undefined) {
        url = await bigBrainAPI({
          ctx,
          method: "POST",
          url: "deployment/url_for_key",
          data: {
            deployKey: configuredAdminKey,
          },
        });
      }
      const deploymentType = deploymentTypeFromAdminKey(configuredAdminKey!);
      return {
        adminKey: configuredAdminKey,
        url,
        deploymentName,
        deploymentType,
      };
    }
    case "urlWithLogin":
      return {
        ...(await bigBrainAPI({
          ctx,
          method: "POST",
          url: "deployment/authorize_within_current_project",
          data: {
            selectedDeploymentName: configuredDeployment,
            projectSelection: await projectSelection(
              ctx,
              configuredDeployment,
              configuredAdminKey,
            ),
          },
        })),
        url: deploymentSelection.url,
      };
    default: {
      const _exhaustivenessCheck: never = deploymentSelection;
      return ctx.crash(1);
    }
  }
}

// Run, Import
export async function fetchDeploymentCredentialsProvisionProd(
  ctx: Context,
  deploymentSelection: DeploymentSelection,
): Promise<{
  url: string;
  adminKey: AdminKey;
  deploymentName?: string;
  deploymentType?: string;
}> {
  if (
    deploymentSelection.kind === "ownDev" &&
    !(await checkAuthorization(ctx, false))
  ) {
    await performLogin(ctx);
  }

  if (deploymentSelection.kind !== "ownDev") {
    const result = await fetchDeploymentCredentialsWithinCurrentProject(
      ctx,
      deploymentSelection,
    );
    logVerbose(
      ctx,
      `Deployment URL: ${result.url}, Deployment Name: ${result.deploymentName}, Deployment Type: ${result.deploymentType}`,
    );
    return {
      url: result.url,
      adminKey: result.adminKey,
      deploymentName: result.deploymentName,
      deploymentType: result.deploymentType,
    };
  }

  const configuredDeployment = await getConfiguredDeploymentOrCrash(ctx);
  const result = await fetchExistingDevDeploymentCredentialsOrCrash(
    ctx,
    configuredDeployment,
  );
  logVerbose(
    ctx,
    `Deployment URL: ${result.url}, Deployment Name: ${configuredDeployment}, Deployment Type: ${result.deploymentType}`,
  );
  return {
    url: result.url,
    adminKey: result.adminKey,
    deploymentType: result.deploymentType,
    deploymentName: configuredDeployment,
  };
}

export async function fetchTeamAndProject(
  ctx: Context,
  deploymentName: string,
) {
  const data = (await bigBrainAPI({
    ctx,
    method: "GET",
    url: `deployment/${deploymentName}/team_and_project`,
  })) as {
    team: string; // slug
    project: string; // slug
    teamId: number;
    projectId: number;
  };

  const { team, project } = data;
  if (team === undefined || project === undefined) {
    const msg =
      "Unknown error when fetching team and project: " + JSON.stringify(data);
    logFailure(ctx, msg);
    return await ctx.crash(1, "transient", new Error(msg));
  }

  return data;
}

// Used by dev for upgrade from team and project in convex.json to CONVEX_DEPLOYMENT
export async function fetchDeploymentCredentialsProvisioningDevOrProdMaybeThrows(
  ctx: Context,
  { teamSlug, projectSlug }: { teamSlug: string; projectSlug: string },
  deploymentType: DeploymentType,
): Promise<{
  deploymentName: string;
  deploymentUrl: string;
  adminKey: AdminKey;
}> {
  const data = await bigBrainAPIMaybeThrows({
    ctx,
    method: "POST",
    url: "deployment/provision_and_authorize",
    data: {
      teamSlug,
      projectSlug,
      deploymentType,
    },
  });
  const deploymentName = data.deploymentName;
  const adminKey = data.adminKey;
  const url = data.url;
  if (adminKey === undefined || url === undefined) {
    const msg = "Unknown error during authorization: " + JSON.stringify(data);
    logError(ctx, chalk.red(msg));
    return await ctx.crash(1, "transient", new Error(msg));
  }
  return { adminKey, deploymentUrl: url, deploymentName };
}

type Credentials = {
  url: string;
  adminKey: AdminKey;
  deploymentType: DeploymentType;
};

type DevCredentials = Credentials & {
  deploymentType: "dev";
};

function credentialsAsDevCredentials(cred: Credentials): DevCredentials {
  if (cred.deploymentType === "dev") {
    return cred as DevCredentials;
  }
  // Getting this wrong is a programmer error.
  // eslint-disable-next-line no-restricted-syntax
  throw new Error("Credentials are not for a dev deployment.");
}

async function fetchExistingDevDeploymentCredentialsOrCrash(
  ctx: Context,
  deploymentName: DeploymentName,
): Promise<DevCredentials> {
  const credentials = await fetchDeploymentCredentialsForName(
    ctx,
    deploymentName,
    "dev",
  );
  if ("error" in credentials) {
    logFailure(
      ctx,
      `Failed to authorize "${deploymentName}" configured in CONVEX_DEPLOYMENT, run \`npx convex dev\` to configure a Convex project`,
    );
    return await ctx.crash(1, "invalid filesystem data", credentials.error);
  }
  if (credentials.deploymentType !== "dev") {
    logFailure(ctx, `Deployment "${deploymentName}" is not a dev deployment`);
    return await ctx.crash(1, "invalid filesystem data");
  }
  return credentialsAsDevCredentials(credentials);
}

// This returns the the url of the deployment from an admin key in the format
//      "tall-forest-1234|1a2b35123541"
//   or "prod:tall-forest-1234|1a2b35123541"
async function deriveUrlFromAdminKey(ctx: Context, adminKey: string) {
  const deploymentName = await deploymentNameFromAdminKeyOrCrash(ctx, adminKey);
  return `https://${deploymentName}.convex.cloud`;
}
