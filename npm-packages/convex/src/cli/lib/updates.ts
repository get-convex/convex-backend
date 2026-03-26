import path from "path";
import { logMessage } from "../../bundler/log.js";
import type { Context } from "../../bundler/context.js";
import { readProjectConfig } from "./config.js";
import { functionsDir } from "./utils/utils.js";
import { checkAiFilesStaleness } from "./aiFiles/index.js";
import { getVersion } from "./versionApi.js";

/**
 * Check the version of the `convex` NPM package and nag if Convex AI files
 * are out of date.
 */
export async function checkVersion(ctx: Context) {
  const version = await getVersion();

  if (version.kind === "error") {
    return;
  }

  if (version.data.message) {
    logMessage(version.data.message);
  }

  try {
    const { configPath, projectConfig } = await readProjectConfig(ctx);
    const convexDir = path.resolve(functionsDir(configPath, projectConfig));
    const projectDir = path.resolve(path.dirname(configPath));
    await checkAiFilesStaleness({
      canonicalGuidelinesHash: version.data.guidelinesHash,
      canonicalAgentSkillsSha: version.data.agentSkillsSha,
      projectDir,
      convexDir,
    });
  } catch {
    // Non-fatal: skip staleness check if project config can't be resolved.
  }
}
