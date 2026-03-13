import path from "path";
import { logMessage } from "../../bundler/log.js";
import type { Context } from "../../bundler/context.js";
import { readProjectConfig } from "./config.js";
import { functionsDir } from "./utils/utils.js";
import { checkAiFilesStaleness } from "./ai/index.js";
import { getVersion } from "./versionApi.js";

/**
 * Check the version of the `convex` NPM package and nag if Convex AI files
 * are out of date.
 */
export async function checkVersion(ctx: Context) {
  const version = await getVersion();

  if (version === null) {
    return;
  }

  if (version.message) {
    logMessage(version.message);
  }

  try {
    const { configPath, projectConfig } = await readProjectConfig(ctx);
    const convexDir = path.resolve(functionsDir(configPath, projectConfig));
    const projectDir = path.resolve(path.dirname(configPath));
    await checkAiFilesStaleness(
      version.guidelinesHash,
      version.agentSkillsSha,
      projectDir,
      convexDir,
    );
  } catch {
    // Non-fatal: skip staleness check if project config can't be resolved.
  }
}
