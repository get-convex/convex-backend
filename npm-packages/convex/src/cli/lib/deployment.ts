import * as dotenv from "dotenv";
import { Context } from "../../bundler/context.js";
import { changedEnvVarFile, getEnvVarRegex } from "./envvars.js";
import {
  CONVEX_DEPLOY_KEY_ENV_VAR_NAME,
  ENV_VAR_FILE_PATH,
  readAdminKeyFromEnvVar,
} from "./utils/utils.js";
import { DeploymentType } from "./api.js";

export const CONVEX_DEPLOYMENT_VAR_NAME = "CONVEX_DEPLOYMENT";

// Return the "target" deployment name, from admin key or from CONVEX_DEPLOYMENT.
// Admin key is set either via a CLI option or via CONVEX_DEPLOY_KEY
export function getTargetDeploymentName() {
  return (
    getDeploymentNameFromAdminKey() ?? getConfiguredDeploymentFromEnvVar().name
  );
}

export function getConfiguredDeploymentFromEnvVar(): {
  type: "dev" | "prod" | "preview" | "local" | null;
  name: string | null;
} {
  dotenv.config({ path: ENV_VAR_FILE_PATH });
  dotenv.config();
  const raw = process.env[CONVEX_DEPLOYMENT_VAR_NAME] ?? null;
  if (raw === null || raw === "") {
    return { type: null, name: null };
  }
  const name = stripDeploymentTypePrefix(raw);
  const type = getDeploymentTypeFromConfiguredDeployment(raw);
  return { type, name };
}

// Given a deployment string like "dev:tall-forest-1234"
// returns only the slug "tall-forest-1234".
// If there's no prefix returns the original string.
export function stripDeploymentTypePrefix(deployment: string) {
  return deployment.split(":").at(-1)!;
}

// Handling legacy CONVEX_DEPLOYMENT without type prefix as well
function getDeploymentTypeFromConfiguredDeployment(raw: string) {
  const typeRaw = raw.split(":")[0];
  const type =
    typeRaw === "prod" ||
    typeRaw === "dev" ||
    typeRaw === "preview" ||
    typeRaw === "local"
      ? typeRaw
      : null;
  return type;
}

export async function writeDeploymentEnvVar(
  ctx: Context,
  deploymentType: DeploymentType,
  deployment: { team: string; project: string; deploymentName: string },
): Promise<{ wroteToGitIgnore: boolean; changedDeploymentEnvVar: boolean }> {
  const existingFile = ctx.fs.exists(ENV_VAR_FILE_PATH)
    ? ctx.fs.readUtf8File(ENV_VAR_FILE_PATH)
    : null;
  const changedFile = changesToEnvVarFile(
    existingFile,
    deploymentType,
    deployment,
  );
  // Also update process.env directly, because `dotfile.config()` doesn't pick
  // up changes to the file.
  const existingValue = process.env[CONVEX_DEPLOYMENT_VAR_NAME];
  const deploymentEnvVarValue =
    deploymentType + ":" + deployment.deploymentName;
  process.env[CONVEX_DEPLOYMENT_VAR_NAME] = deploymentEnvVarValue;

  if (changedFile !== null) {
    ctx.fs.writeUtf8File(ENV_VAR_FILE_PATH, changedFile);
    // Only do this if we're not reinitializing an existing setup
    return {
      wroteToGitIgnore: await gitIgnoreEnvVarFile(ctx),
      changedDeploymentEnvVar: existingValue !== deploymentEnvVarValue,
    };
  }
  return {
    wroteToGitIgnore: false,
    changedDeploymentEnvVar: existingValue !== deploymentEnvVarValue,
  };
}

// Only used in the internal --url flow
export async function eraseDeploymentEnvVar(ctx: Context): Promise<boolean> {
  const existingFile = ctx.fs.exists(ENV_VAR_FILE_PATH)
    ? ctx.fs.readUtf8File(ENV_VAR_FILE_PATH)
    : null;
  if (existingFile === null) {
    return false;
  }
  const config = dotenv.parse(existingFile);
  const existing = config[CONVEX_DEPLOYMENT_VAR_NAME];
  if (existing === undefined) {
    return false;
  }
  const changedFile = existingFile.replace(
    getEnvVarRegex(CONVEX_DEPLOYMENT_VAR_NAME),
    "",
  );
  ctx.fs.writeUtf8File(ENV_VAR_FILE_PATH, changedFile);
  return true;
}

async function gitIgnoreEnvVarFile(ctx: Context): Promise<boolean> {
  const gitIgnorePath = ".gitignore";
  const gitIgnoreContents = ctx.fs.exists(gitIgnorePath)
    ? ctx.fs.readUtf8File(gitIgnorePath)
    : "";
  const changedGitIgnore = changesToGitIgnore(gitIgnoreContents);
  if (changedGitIgnore !== null) {
    ctx.fs.writeUtf8File(gitIgnorePath, changedGitIgnore);
    return true;
  }
  return false;
}

// exported for tests
export function changesToEnvVarFile(
  existingFile: string | null,
  deploymentType: DeploymentType,
  {
    team,
    project,
    deploymentName,
  }: { team: string; project: string; deploymentName: string },
): string | null {
  const deploymentValue = deploymentType + ":" + deploymentName;
  const commentOnPreviousLine = "# Deployment used by `npx convex dev`";
  const commentAfterValue = `team: ${team}, project: ${project}`;
  return changedEnvVarFile(
    existingFile,
    CONVEX_DEPLOYMENT_VAR_NAME,
    deploymentValue,
    commentAfterValue,
    commentOnPreviousLine,
  );
}

// exported for tests
export function changesToGitIgnore(existingFile: string | null): string | null {
  if (existingFile === null) {
    return `${ENV_VAR_FILE_PATH}\n`;
  }
  const gitIgnoreLines = existingFile.split("\n");
  const envVarFileIgnored = gitIgnoreLines.some((line) => {
    if (line.startsWith("#")) return false;
    if (line.startsWith("!")) return false;

    // .gitignore ignores trailing whitespace, and also we need to remove
    // the trailing `\r` from Windows-style newline since we split on `\n`.
    const trimmedLine = line.trimEnd();

    const envIgnorePatterns = [
      /^\.env\.local$/,
      /^\.env\.\*$/,
      /^\.env\*$/,
      /^.*\.local$/,
      /^\.env\*\.local$/,
    ];

    return envIgnorePatterns.some((pattern) => pattern.test(trimmedLine));
  });
  if (!envVarFileIgnored) {
    return `${existingFile}\n${ENV_VAR_FILE_PATH}\n`;
  } else {
    return null;
  }
}

export function getDeploymentNameFromAdminKey() {
  const adminKey = readAdminKeyFromEnvVar();
  if (adminKey === undefined) {
    return null;
  }
  return deploymentNameFromAdminKey(adminKey);
}

export async function deploymentNameFromAdminKeyOrCrash(
  ctx: Context,
  adminKey: string,
) {
  const deploymentName = deploymentNameFromAdminKey(adminKey);
  if (deploymentName === null) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Please set ${CONVEX_DEPLOY_KEY_ENV_VAR_NAME} to a new key which you can find on your Convex dashboard.`,
    });
  }
  return deploymentName;
}

function deploymentNameFromAdminKey(adminKey: string) {
  const parts = adminKey.split("|");
  if (parts.length === 1) {
    return null;
  }
  if (isPreviewDeployKey(adminKey)) {
    // Preview deploy keys do not contain a deployment name.
    return null;
  }
  return stripDeploymentTypePrefix(parts[0]);
}

// Needed to differentiate a preview deploy key
// from a concrete preview deployment's deploy key.
// preview deploy key: `preview:team:project|key`
// preview deployment's deploy key: `preview:deploymentName|key`
export function isPreviewDeployKey(adminKey: string) {
  const parts = adminKey.split("|");
  if (parts.length === 1) {
    return false;
  }
  const [prefix] = parts;
  const prefixParts = prefix.split(":");
  return prefixParts[0] === "preview" && prefixParts.length === 3;
}

// For current keys returns prod|dev|preview,
// for legacy keys returns "prod".
// Examples:
//  "prod:deploymentName|key" -> "prod"
//  "preview:deploymentName|key" -> "preview"
//  "dev:deploymentName|key" -> "dev"
//  "key" -> "prod"
export function deploymentTypeFromAdminKey(adminKey: string) {
  const parts = adminKey.split(":");
  if (parts.length === 1) {
    return "prod";
  }
  return parts.at(0)!;
}

export async function getTeamAndProjectFromPreviewAdminKey(
  ctx: Context,
  adminKey: string,
) {
  const parts = adminKey.split("|")[0].split(":");
  if (parts.length !== 3) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage:
        "Malformed preview CONVEX_DEPLOY_KEY, get a new key from Project Settings.",
    });
  }
  const [_preview, teamSlug, projectSlug] = parts;
  return { teamSlug, projectSlug };
}

export type OnDeploymentActivityFunc = (
  isOffline: boolean,
  wasOffline: boolean,
) => Promise<void>;
export type CleanupDeploymentFunc = () => Promise<void>;
export type DeploymentDetails = {
  deploymentName: string;
  deploymentUrl: string;
  adminKey: string;
  onActivity: OnDeploymentActivityFunc | null;
};
