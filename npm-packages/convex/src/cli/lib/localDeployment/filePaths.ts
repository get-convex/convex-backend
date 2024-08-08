/*
~/.convex
  binaries
    convex-backend.zip
    0.0.1
      convex-local-backend
    0.0.2
      convex-local-backend
  convex-backend-state
    local-my_team-chess
      config.json // contains `LocalDeploymentConfig`
      convex_local_storage
      convex_local_backend.sqlite3
    local-my_team-whisper
      config.json
      convex_local_storage
      convex_local_backend.sqlite3
*/

import path from "path";
import { rootDirectory } from "../utils.js";
import { Context } from "../../../bundler/context.js";

export function rootDeploymentStateDir() {
  return path.join(rootDirectory(), "convex-backend-state");
}

export function deploymentStateDir(deploymentName: string) {
  return path.join(rootDeploymentStateDir(), deploymentName);
}

export type LocalDeploymentConfig = {
  ports: {
    cloud: number;
    site: number;
  };
  backendVersion: string;
  adminKey: string;
};
export function loadDeploymentConfig(
  ctx: Context,
  deploymentName: string,
): LocalDeploymentConfig | null {
  const configFile = path.join(
    deploymentStateDir(deploymentName),
    "config.json",
  );
  if (ctx.fs.exists(configFile)) {
    return JSON.parse(ctx.fs.readUtf8File(configFile));
  }
  return null;
}

export function saveDeploymentConfig(
  ctx: Context,
  deploymentName: string,
  config: LocalDeploymentConfig,
) {
  const configFile = path.join(
    deploymentStateDir(deploymentName),
    "config.json",
  );
  if (!ctx.fs.exists(deploymentStateDir(deploymentName))) {
    ctx.fs.mkdir(deploymentStateDir(deploymentName), { recursive: true });
  }
  ctx.fs.writeUtf8File(configFile, JSON.stringify(config));
}

export function binariesDir() {
  return path.join(rootDirectory(), "binaries");
}

export function binaryZip() {
  return path.join(binariesDir(), "convex-backend.zip");
}

export function versionedBinaryDir(version: string) {
  return path.join(binariesDir(), version);
}

export function executablePath(version: string) {
  return path.join(versionedBinaryDir(version), "convex-local-backend");
}
