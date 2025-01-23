import { Context, logVerbose } from "../../bundler/context.js";
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
} from "./utils/utils.js";

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
    partitionId,
    deploymentTypeToProvision,
  }: {
    teamSlug: string;
    projectName: string;
    partitionId?: number;
    deploymentTypeToProvision: DeploymentType;
  },
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
    deploymentType: deploymentTypeToProvision,
    partitionId,
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
    return await ctx.crash({
      exitCode: 1,
      errorType: "transient",
      errForSentry: error,
      printedMessage: error,
    });
  }
  return {
    projectSlug,
    teamSlug,
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
    return await ctx.crash({
      exitCode: 1,
      errorType: "transient",
      errForSentry: new Error(msg),
      printedMessage: msg,
    });
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
  | { kind: "ownProd"; partitionId?: number | undefined }
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
  partitionId?: string | undefined;
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
  const partitionId = options.partitionId
    ? parseInt(options.partitionId)
    : undefined;
  return {
    kind: options.prod === true ? "ownProd" : "ownDev",
    partitionId,
  };
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
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage:
          `${buildEnvironmentExpectsConvexDeployKey} build environment detected but ${CONVEX_DEPLOY_KEY_ENV_VAR_NAME} is not set. ` +
          `Set this environment variable to deploy from this environment. See https://docs.convex.dev/production/hosting`,
      });
    }
    const header = await getAuthHeaderForBigBrain(ctx);
    if (!header) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage:
          `Error: You are not logged in. Log in with \`npx convex dev\` or set the ${CONVEX_DEPLOY_KEY_ENV_VAR_NAME} environment variable. ` +
          `See https://docs.convex.dev/production/hosting`,
      });
    }
    const configuredDeployment = await getConfiguredDeploymentName(ctx);
    if (configuredDeployment === null) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage:
          "No CONVEX_DEPLOYMENT set, run `npx convex dev` to configure a Convex project",
      });
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
    return await ctx.crash({
      exitCode: 1,
      errorType: "transient",
      errForSentry: new Error(msg),
      printedMessage: msg,
    });
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
  return await ctx.crash({
    exitCode: 1,
    errorType: "fatal",
    printedMessage:
      "Select project by setting `CONVEX_DEPLOYMENT` with `npx convex dev` or `CONVEX_DEPLOY_KEY` from the Convex dashboard.",
  });
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
          partitionId: deploymentSelection.partitionId,
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
      return ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        // This should be unreachable, so don't bother with a printed message.
        printedMessage: null,
        errForSentry: `Unexpected deployment selection: ${deploymentSelection as any}`,
      });
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
    return await ctx.crash({
      exitCode: 1,
      errorType: "transient",
      errForSentry: new Error(msg),
      printedMessage: msg,
    });
  }

  return data;
}

// Used by dev for upgrade from team and project in convex.json to CONVEX_DEPLOYMENT
export async function fetchDeploymentCredentialsProvisioningDevOrProdMaybeThrows(
  ctx: Context,
  { teamSlug, projectSlug }: { teamSlug: string; projectSlug: string },
  deploymentType: DeploymentType,
  partitionId: number | undefined,
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
      partitionId,
    },
  });
  const deploymentName = data.deploymentName;
  const adminKey = data.adminKey;
  const url = data.url;
  if (adminKey === undefined || url === undefined) {
    const msg = "Unknown error during authorization: " + JSON.stringify(data);
    return await ctx.crash({
      exitCode: 1,
      errorType: "transient",
      errForSentry: new Error(msg),
      printedMessage: msg,
    });
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
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem data",
      errForSentry: credentials.error,
      printedMessage: `Failed to authorize "${deploymentName}" configured in CONVEX_DEPLOYMENT, run \`npx convex dev\` to configure a Convex project`,
    });
  }
  if (credentials.deploymentType !== "dev") {
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem data",
      printedMessage: `Deployment "${deploymentName}" is not a dev deployment`,
    });
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
