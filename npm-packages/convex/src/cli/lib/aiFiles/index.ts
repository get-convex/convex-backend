import * as Sentry from "@sentry/node";
import path from "path";
import { Context } from "../../../bundler/context.js";
// eslint-disable-next-line no-restricted-imports
import { promises as fs } from "fs";
import { chalkStderr } from "chalk";
import { logMessage } from "../../../bundler/log.js";
import { promptYesNo } from "../utils/prompts.js";
import { type AiFilesPaths, aiDirForConvexDir } from "./paths.js";
import {
  installGuidelinesFile,
  hasGuidelinesInstalled,
} from "./guidelinesmd.js";
import {
  type AiFilesConfig,
  hasAiFilesConfig,
  readAiConfig,
  writeAiConfig,
  writeAiEnabledToProjectConfig,
} from "./config.js";
import { isInInteractiveTerminal } from "./utils.js";
import {
  hasAgentsMdInstalled,
  applyAgentsMdSection,
  removeAgentsMdSection,
} from "./agentsmd.js";
import {
  hasClaudeMdInstalled,
  applyClaudeMdSection,
  removeClaudeMdSection,
} from "./claudemd.js";
import { installSkills, removeInstalledSkills } from "./skills.js";
import { removeLegacyCursorRulesFile as removeLegacyCursorRules } from "./cursorrules.js";
async function hasExistingAiFilesArtifacts({
  projectDir,
  convexDir,
}: AiFilesPaths): Promise<boolean> {
  return (
    (await hasGuidelinesInstalled(convexDir)) ||
    (await hasAgentsMdInstalled(projectDir)) ||
    (await hasClaudeMdInstalled(projectDir))
  );
}

/**
 * Install or refresh all Convex AI files.
 *
 * Reads the existing config if present, or starts from a blank one for a
 * fresh install. Each component can be individually skipped via the optional
 * flags (all default to true).
 */
export async function installAiFiles({
  projectDir,
  convexDir,
  shouldWriteGuidelines = true,
  shouldWriteAgentsMd = true,
  shouldWriteClaudeMd = true,
  shouldWriteSkills = true,
}: AiFilesPaths & {
  shouldWriteGuidelines?: boolean;
  shouldWriteAgentsMd?: boolean;
  shouldWriteClaudeMd?: boolean;
  shouldWriteSkills?: boolean;
}): Promise<void> {
  await fs.mkdir(aiDirForConvexDir(convexDir), { recursive: true });

  const config: AiFilesConfig = (await readAiConfig({
    projectDir,
    convexDir,
  })) ?? {
    enabled: true,
    guidelinesHash: null,
    agentsMdSectionHash: null,
    claudeMdHash: null,
    agentSkillsSha: null,
    installedSkillNames: [],
  };

  if (shouldWriteGuidelines) await installGuidelinesFile({ convexDir, config });

  const convexDirName = path.relative(projectDir, convexDir);

  if (shouldWriteAgentsMd)
    await applyAgentsMdSection({ projectDir, config, convexDirName });

  if (shouldWriteClaudeMd)
    await applyClaudeMdSection({ projectDir, config, convexDirName });

  if (shouldWriteSkills) await installSkills({ projectDir, config });

  await removeLegacyCursorRules(projectDir);
  await writeAiConfig({ config, projectDir, convexDir });

  logMessage(`${chalkStderr.green("✔")} Convex AI files installed.`);
}

async function attemptToInstallAiFiles(
  opts: Parameters<typeof installAiFiles>[0],
): Promise<void> {
  try {
    await installAiFiles(opts);
  } catch (error) {
    Sentry.captureException(error);
  }
}

type AiFilesStalenessStatus =
  | "not-installed" // no config AND no artifacts — show install nag
  | "has-artifacts" // no config but files exist on disk (e.g. fresh checkout) — stay quiet
  | "disabled" // user opted out of nag messages
  | "stale" // one or more files are out of date
  | "up-to-date"; // everything looks fine

async function determineAiFilesStaleness({
  canonicalGuidelinesHash,
  canonicalAgentSkillsSha,
  projectDir,
  convexDir,
}: {
  canonicalGuidelinesHash: string | null;
  canonicalAgentSkillsSha: string | null;
} & AiFilesPaths): Promise<AiFilesStalenessStatus> {
  const config = await readAiConfig({ projectDir, convexDir });

  if (config === null) {
    const hasArtifacts = await hasExistingAiFilesArtifacts({
      projectDir,
      convexDir,
    });
    return hasArtifacts ? "has-artifacts" : "not-installed";
  }

  if (!config.enabled) return "disabled";

  if (canonicalGuidelinesHash === null && canonicalAgentSkillsSha === null)
    return "up-to-date";

  const guidelinesStale =
    canonicalGuidelinesHash !== null &&
    config.guidelinesHash !== null &&
    config.guidelinesHash !== canonicalGuidelinesHash;

  const skillsStale =
    canonicalAgentSkillsSha !== null &&
    config.agentSkillsSha !== null &&
    config.agentSkillsSha !== canonicalAgentSkillsSha;

  return guidelinesStale || skillsStale ? "stale" : "up-to-date";
}

/**
 * Check whether the Convex AI files are out of date and log a nag message
 * if so.
 */
export async function checkAiFilesStaleness(
  opts: {
    canonicalGuidelinesHash: string | null;
    canonicalAgentSkillsSha: string | null;
  } & AiFilesPaths,
): Promise<void> {
  const status = await determineAiFilesStaleness(opts);

  if (status === "not-installed") {
    logMessage(
      chalkStderr.yellow(
        `Convex AI files are not installed. Run ${chalkStderr.bold(`npx convex ai-files install`)} to get started or ${chalkStderr.bold(`npx convex ai-files disable`)} to hide this message.`,
      ),
    );
  }

  if (status === "stale") {
    logMessage(
      chalkStderr.yellow(
        `Your Convex AI files are out of date. Run ${chalkStderr.bold(`npx convex ai-files update`)} to get the latest.`,
      ),
    );
  }
}

export async function enableAiFiles({
  projectDir,
  convexDir,
}: AiFilesPaths): Promise<void> {
  await installAiFiles({ projectDir, convexDir });
  const config = await readAiConfig({ projectDir, convexDir });
  if (config === null) return;
  config.enabled = true;
  await writeAiConfig({
    config,
    projectDir,
    convexDir,
    options: { persistEnabledPreference: "always" },
  });
}

/**
 * Remove all Convex AI files from the project.
 * Called by `npx convex ai-files remove`.
 */
export async function removeAiFiles({
  projectDir,
  convexDir,
}: AiFilesPaths): Promise<void> {
  const config = await readAiConfig({ projectDir, convexDir });
  if (config === null) {
    logMessage("No Convex AI files found — nothing to remove.");
    return;
  }

  const removals = [
    await removeAgentsMdSection(projectDir),
    await removeClaudeMdSection(projectDir),
    await removeInstalledSkills({
      projectDir,
      skillNames: config.installedSkillNames,
    }),
    await removeLegacyCursorRules(projectDir),
    await attemptToDeleteAiDir({ projectDir, convexDir }),
  ];

  if (removals.some(Boolean)) logMessage("Convex AI files removed.");
}

/**
 * Called by `npx convex ai-files disable`.
 *
 * Writes a suppression flag into `convex.json` so `npx convex dev` stops
 * showing AI files install/staleness messages. Files are left in place.
 */
export async function safelyAttemptToDisableAiFiles(
  projectDir: string,
): Promise<void> {
  try {
    await writeAiEnabledToProjectConfig({
      projectDir,
      enabled: false,
    });
    logMessage(
      `${chalkStderr.green(`✔`)} Convex AI files disabled. Run ${chalkStderr.bold(`npx convex ai-files enable`)} to re-enable.`,
    );
  } catch (error) {
    Sentry.captureException(error);
    logMessage(
      chalkStderr.yellow(
        "Could not write AI message suppression config. Message may reappear.",
      ),
    );
  }
}

async function attemptToDeleteAiDir({
  projectDir,
  convexDir,
}: AiFilesPaths): Promise<boolean> {
  const aiDir = aiDirForConvexDir(convexDir);
  const relPath = path.relative(projectDir, aiDir);
  try {
    await fs.rm(aiDir, { recursive: true, force: true });
    logMessage(`${chalkStderr.green("✔")} Deleted ${relPath}/`);
    return true;
  } catch (error) {
    Sentry.captureException(error);
    logMessage(
      chalkStderr.yellow(`Could not delete ${relPath}/. Remove it manually.`),
    );
    return false;
  }
}

async function hasAiFilesBeenInstalledBefore({
  projectDir,
  convexDir,
}: AiFilesPaths): Promise<boolean> {
  return (
    (await hasAiFilesConfig({ projectDir, convexDir })) ||
    (await hasExistingAiFilesArtifacts({ projectDir, convexDir }))
  );
}

export async function maybeSetupAiFiles({
  ctx,
  convexDir,
  projectDir,
}: {
  ctx: Context;
} & AiFilesPaths): Promise<void> {
  if (!isInInteractiveTerminal()) return;

  const config = await readAiConfig({ projectDir, convexDir });
  if (config !== null && !config.enabled) return;

  if (await hasAiFilesBeenInstalledBefore({ projectDir, convexDir })) {
    await attemptToInstallAiFiles({ projectDir, convexDir });
    return;
  }

  const shouldInstall = await promptYesNo(ctx, {
    message: "Set up Convex AI files? (guidelines, AGENTS.md, agent skills)",
    default: true,
  });

  if (shouldInstall) await attemptToInstallAiFiles({ projectDir, convexDir });
}
