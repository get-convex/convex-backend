import * as Sentry from "@sentry/node";
import child_process from "child_process";
import path from "path";
import { Context } from "../../../bundler/context.js";
// Use raw fs (not ctx.fs) so these operations run asynchronously and don't
// interfere with the file-watcher used by `convex dev`.
// eslint-disable-next-line no-restricted-imports
import { promises as fs } from "fs";
import { chalkStderr } from "chalk";
import { logMessage } from "../../../bundler/log.js";
import {
  AGENTS_MD_START_MARKER,
  AGENTS_MD_END_MARKER,
  agentsMdConvexSection,
} from "../../codegen_templates/agentsmd.js";
import {
  CLAUDE_MD_END_MARKER,
  CLAUDE_MD_START_MARKER,
  claudeMdConvexSection,
} from "../../codegen_templates/claudemd.js";
import {
  downloadGuidelines,
  fetchAgentSkillsSha,
  getVersion,
} from "../versionApi.js";
import { promptYesNo } from "../utils/prompts.js";
import { hashSha256 } from "../utils/hash.js";
import {
  aiDirForConvexDir,
  agentsMdPath,
  claudeMdPath,
  guidelinesPathForConvexDir,
} from "./paths.js";
import {
  type AiFilesConfig,
  readAiConfig,
  writeAiConfig,
  writeAiDisabledToProjectConfig,
} from "./config.js";

function isAgentMode(): boolean {
  return process.env.CONVEX_AGENT_MODE !== undefined;
}

// ---------------------------------------------------------------------------
// AGENTS.md helpers
// ---------------------------------------------------------------------------

export async function injectAgentsMdSection(
  section: string,
  projectDir?: string,
): Promise<string | null> {
  const filePath = agentsMdPath(projectDir);
  let existing = "";
  try {
    existing = await fs.readFile(filePath, "utf8");
  } catch {
    // File doesn't exist — we'll create it.
  }

  let updated: string;
  const startIdx = existing.indexOf(AGENTS_MD_START_MARKER);
  const endIdx = existing.indexOf(AGENTS_MD_END_MARKER);

  if (startIdx !== -1 && endIdx !== -1) {
    // Replace existing Convex section.
    updated =
      existing.slice(0, startIdx) +
      section +
      existing.slice(endIdx + AGENTS_MD_END_MARKER.length);
  } else if (existing.length > 0) {
    // Append to existing file (with a blank line separator).
    updated = existing.trimEnd() + "\n\n" + section + "\n";
  } else {
    // Create new file.
    updated = section + "\n";
  }

  await fs.writeFile(filePath, updated, "utf8");
  return hashSha256(section);
}

type InjectClaudeSectionResult = {
  sectionHash: string;
  didWrite: boolean;
};

export async function injectClaudeMdSection(
  section: string,
  projectDir?: string,
): Promise<InjectClaudeSectionResult> {
  const filePath = claudeMdPath(projectDir);
  let existing = "";
  try {
    existing = await fs.readFile(filePath, "utf8");
  } catch {
    // File doesn't exist - we'll create it.
  }

  let updated: string;
  const startIdx = existing.indexOf(CLAUDE_MD_START_MARKER);
  const endIdx = existing.indexOf(CLAUDE_MD_END_MARKER);

  if (startIdx !== -1 && endIdx !== -1) {
    // Replace existing Convex section.
    updated =
      existing.slice(0, startIdx) +
      section +
      existing.slice(endIdx + CLAUDE_MD_END_MARKER.length);
  } else if (existing.length > 0) {
    // Append to existing file (with a blank line separator).
    updated = existing.trimEnd() + "\n\n" + section + "\n";
  } else {
    // Create new file.
    updated = section + "\n";
  }

  const didWrite = updated !== existing;
  if (didWrite) {
    await fs.writeFile(filePath, updated, "utf8");
  }
  return {
    sectionHash: hashSha256(section),
    didWrite,
  };
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Write/update all Convex AI files (guidelines, AGENTS.md, CLAUDE.md, skills).
 *
 * @param convexDir - absolute path to the project's convex functions directory
 *   (e.g. `/home/user/myapp/convex`). Used to build the `_generated/ai/` subdirectory.
 */
export async function writeAiFiles(
  convexDir: string,
  installSkills: boolean = false,
  skillsOutputMode: "verbose" | "quiet" = "verbose",
  projectDirOverride?: string,
): Promise<void> {
  const projectDir = path.resolve(
    projectDirOverride ?? path.dirname(convexDir),
  );

  try {
    // Ensure convex/_generated/ai/ directory exists.
    await fs.mkdir(aiDirForConvexDir(convexDir), { recursive: true });

    const config: AiFilesConfig = {
      disableStalenessMessage: false,
      guidelinesHash: null,
      agentsMdSectionHash: null,
      claudeMdHash: null,
      agentSkillsSha: null,
      installedSkillNames: [],
    };

    // Write guidelines.
    const guidelines = await downloadGuidelines();
    if (guidelines !== null) {
      await fs.writeFile(
        guidelinesPathForConvexDir(convexDir),
        guidelines,
        "utf8",
      );
      config.guidelinesHash = hashSha256(guidelines);
    } else {
      logMessage(
        chalkStderr.yellow(
          "Could not download Convex AI guidelines right now. You can retry with: npx convex ai-files install",
        ),
      );
    }

    // Inject Convex section into AGENTS.md.
    const convexDirName = path.relative(projectDir, convexDir);
    const section = agentsMdConvexSection(convexDirName);
    config.agentsMdSectionHash = await injectAgentsMdSection(
      section,
      projectDir,
    );

    // Inject Convex section into CLAUDE.md.
    const claudeSection = claudeMdConvexSection(convexDirName);
    const claudeInjectResult = await injectClaudeMdSection(
      claudeSection,
      projectDir,
    );
    config.claudeMdHash = claudeInjectResult.sectionHash;

    if (installSkills) {
      if (await shouldRunSkillsCli()) {
        logMessage("Installing Convex agent skills...");
        const skillsOk = await runSkillsAdd(projectDir, skillsOutputMode);
        if (skillsOk) {
          // Record the canonical SHA for staleness detection.
          const sha = await fetchAgentSkillsSha();
          if (sha) {
            config.agentSkillsSha = sha;
          }
          const names = await readInstalledSkillNames(projectDir);
          if (names.length > 0) {
            config.installedSkillNames = names;
          }
        } else {
          logMessage(
            chalkStderr.yellow(
              "Could not install agent skills. You can retry manually with: npx skills add get-convex/agent-skills",
            ),
          );
        }
      }
    }

    // Clean up the legacy `.cursor/rules/convex_rules.mdc` file if present.
    // It was written by the old cursor rules auto-update and is now superseded
    // by `convex/_generated/ai/guidelines.md`.
    const removedLegacy = await removeLegacyCursorRulesFile(projectDir);
    if (removedLegacy) {
      logMessage(
        `${chalkStderr.green(`✔`)} Removed legacy .cursor/rules/convex_rules.mdc (superseded by convex/_generated/ai/guidelines.md).`,
      );
    }

    await writeAiConfig(config, projectDir, convexDir);

    logMessage(
      `${chalkStderr.green(`✔`)} Wrote Convex AI files to ${path.relative(projectDir, aiDirForConvexDir(convexDir))}/`,
    );
  } catch (error) {
    Sentry.captureException(error);
    // Non-fatal — don't break the init flow.
  }
}

/**
 * Check whether the Convex AI files are out of date and log a nag message
 * if so.
 */
export async function checkAiFilesStaleness(
  canonicalGuidelinesHash: string | null,
  canonicalAgentSkillsSha: string | null,
  projectDir: string,
  convexDir: string,
): Promise<void> {
  const config = await readAiConfig(projectDir, convexDir);

  if (config === null) {
    // No state file means AI files have not been set up yet. Keep nudging until
    // the user either installs them or explicitly disables the nag with
    // `npx convex ai-files disable`.
    logMessage(
      chalkStderr.yellow(
        isAgentMode()
          ? `Convex AI files are not installed. If you are an agent tell the human to run ${chalkStderr.bold(`npx convex ai-files install`)} to get started or ${chalkStderr.bold(`npx convex ai-files disable`)} to hide this message.`
          : `Convex AI files are not installed. Run ${chalkStderr.bold(`npx convex ai-files install`)} to get started or ${chalkStderr.bold(`npx convex ai-files disable`)} to hide this message.`,
      ),
    );
    return;
  }

  if (config.disableStalenessMessage) {
    // User explicitly suppressed staleness/install messages.
    return;
  }

  // No version data from server — skip staleness check (non-fatal).
  if (canonicalGuidelinesHash === null && canonicalAgentSkillsSha === null) {
    return;
  }

  const guidelinesStale =
    canonicalGuidelinesHash !== null &&
    config.guidelinesHash !== null &&
    config.guidelinesHash !== canonicalGuidelinesHash;

  const skillsStale =
    canonicalAgentSkillsSha !== null &&
    config.agentSkillsSha !== null &&
    config.agentSkillsSha !== canonicalAgentSkillsSha;

  if (guidelinesStale || skillsStale) {
    logMessage(
      chalkStderr.yellow(
        `Your Convex AI files are out of date. Run ${chalkStderr.bold(`npx convex ai-files update`)} to get the latest.`,
      ),
    );
  }
}

/**
 * Update all Convex AI files to their latest versions.
 *
 * Files the user has modified (detected via hash comparison) are skipped
 * with a warning rather than silently overwritten.
 *
 * @param projectDir - absolute path to the project root directory.
 * @param convexDir - absolute path to the Convex functions directory.
 */
export async function updateAiFiles(
  projectDir: string,
  convexDir: string,
): Promise<void> {
  const config = await readAiConfig(projectDir, convexDir);
  if (config === null) {
    // No config yet — run the full init and install skills.
    await writeAiFiles(convexDir, true, "verbose", projectDir);
    return;
  }

  // Config can exist (for example, disableStalenessMessage in convex.json) even
  // when convex/_generated/ai/ was removed. Recreate it so reinstall/update
  // paths do not fail when writing guidelines/state.
  await fs.mkdir(aiDirForConvexDir(convexDir), { recursive: true });

  let updatedCount = 0;
  let skippedCount = 0;

  // Update guidelines.
  const guidelines = await downloadGuidelines();
  if (guidelines !== null) {
    const newHash = hashSha256(guidelines);
    if (newHash === config.guidelinesHash) {
      logMessage("Convex AI guidelines are already up to date.");
    } else {
      const currentContent = await readFileSafe(
        guidelinesPathForConvexDir(convexDir),
      );
      if (
        currentContent !== null &&
        config.guidelinesHash !== null &&
        hashSha256(currentContent) !== config.guidelinesHash
      ) {
        logMessage(
          chalkStderr.yellow(
            `Skipping ${path.relative(projectDir, guidelinesPathForConvexDir(convexDir))} — file has been modified locally.`,
          ),
        );
        skippedCount++;
      } else {
        await fs.writeFile(
          guidelinesPathForConvexDir(convexDir),
          guidelines,
          "utf8",
        );
        config.guidelinesHash = newHash;
        updatedCount++;
      }
    }
  } else {
    logMessage(
      chalkStderr.yellow(
        "Could not download Convex AI guidelines right now. Keeping your existing guidelines file.",
      ),
    );
  }

  // Update AGENTS.md section. The dir name is hardcoded to match paths.ts
  // (the only non-standard case was create-react-app which is dead).
  const convexDirName = path.relative(projectDir, convexDir);
  const section = agentsMdConvexSection(convexDirName);
  const newSectionHash = hashSha256(section);

  if (newSectionHash !== config.agentsMdSectionHash) {
    config.agentsMdSectionHash = await injectAgentsMdSection(
      section,
      projectDir,
    );
    updatedCount++;
  }

  // The skills CLI streams its own output, so we don't count this in
  // updatedCount.
  if (await shouldRunSkillsCli()) {
    logMessage("Installing Convex agent skills...");
    const skillsOk = await runSkillsAdd(projectDir);
    if (skillsOk) {
      // Record the canonical SHA for staleness detection.
      const sha = await fetchAgentSkillsSha();
      if (sha) {
        config.agentSkillsSha = sha;
      }
      // Track installed skill names so `convex ai-files remove` can pass them
      // to `npx skills remove`.
      const names = await readInstalledSkillNames(projectDir);
      if (names.length > 0) {
        config.installedSkillNames = names;
      }
    } else {
      logMessage(
        chalkStderr.yellow(
          "Could not install agent skills. You can retry manually with: npx skills add get-convex/agent-skills",
        ),
      );
    }
  }

  // Clean up the legacy `.cursor/rules/convex_rules.mdc` file if present.
  // It was written by the old cursor rules auto-update and is now superseded
  // by `convex/_generated/ai/guidelines.md`.
  const removedLegacy = await removeLegacyCursorRulesFile(projectDir);
  if (removedLegacy) {
    logMessage(
      `${chalkStderr.green(`✔`)} Removed legacy .cursor/rules/convex_rules.mdc (superseded by convex/_generated/ai/guidelines.md).`,
    );
    updatedCount++;
  }

  // Update/inject the Convex-managed CLAUDE.md section.
  const claudeSection = claudeMdConvexSection(convexDirName);
  const claudeInjectResult = await injectClaudeMdSection(
    claudeSection,
    projectDir,
  );
  config.claudeMdHash = claudeInjectResult.sectionHash;
  if (claudeInjectResult.didWrite) {
    updatedCount++;
  }

  await writeAiConfig(config, projectDir, convexDir);

  if (updatedCount > 0) {
    logMessage(
      `${chalkStderr.green(`✔`)} Updated ${updatedCount} Convex AI file${updatedCount === 1 ? "" : "s"}.`,
    );
  }
  if (skippedCount > 0) {
    logMessage(
      chalkStderr.yellow(
        `Skipped ${skippedCount} file${skippedCount === 1 ? "" : "s"} with local modifications.`,
      ),
    );
  }
  if (updatedCount === 0 && skippedCount === 0) {
    logMessage("Convex AI files are already up to date.");
  }
}

export async function enableAiFiles(
  projectDir: string,
  convexDir: string,
): Promise<void> {
  await updateAiFiles(projectDir, convexDir);
  const config = await readAiConfig(projectDir, convexDir);
  if (config === null) {
    return;
  }
  config.disableStalenessMessage = false;
  await writeAiConfig(config, projectDir, convexDir, {
    persistDisabledPreference: "always",
  });
}

/**
 * Remove the Convex-managed section from AGENTS.md (between the start/end
 * markers). If the file becomes empty or only whitespace after removal it is
 * deleted. Returns true if a section was found and removed.
 */
async function stripAgentsMdSection(projectDir: string): Promise<boolean> {
  const filePath = agentsMdPath(projectDir);
  let content: string;
  try {
    content = await fs.readFile(filePath, "utf8");
  } catch {
    return false;
  }
  const startIdx = content.indexOf(AGENTS_MD_START_MARKER);
  const endIdx = content.indexOf(AGENTS_MD_END_MARKER);
  if (startIdx === -1 || endIdx === -1) {
    return false;
  }
  const before = content.slice(0, startIdx).trimEnd();
  const after = content.slice(endIdx + AGENTS_MD_END_MARKER.length).trimStart();
  const updated = [before, after].filter(Boolean).join("\n\n");

  if (!updated.trim()) {
    try {
      await fs.unlink(filePath);
    } catch {
      // Ignore errors
    }
  } else {
    await fs.writeFile(filePath, updated + "\n", "utf8");
  }
  return true;
}

/**
 * Remove the Convex-managed section from CLAUDE.md (between start/end markers).
 * If the file becomes empty after removal, delete it.
 */
async function stripClaudeMdSection(
  projectDir: string,
): Promise<"none" | "section" | "file"> {
  const filePath = claudeMdPath(projectDir);
  let content: string;
  try {
    content = await fs.readFile(filePath, "utf8");
  } catch {
    return "none";
  }
  const startIdx = content.indexOf(CLAUDE_MD_START_MARKER);
  const endIdx = content.indexOf(CLAUDE_MD_END_MARKER);
  if (startIdx === -1 || endIdx === -1) {
    return "none";
  }
  const before = content.slice(0, startIdx).trimEnd();
  const after = content.slice(endIdx + CLAUDE_MD_END_MARKER.length).trimStart();
  const updated = [before, after].filter(Boolean).join("\n\n");

  if (!updated.trim()) {
    try {
      await fs.unlink(filePath);
    } catch {
      // Ignore errors.
    }
    return "file";
  }
  await fs.writeFile(filePath, updated + "\n", "utf8");
  return "section";
}

/**
 * Remove all Convex AI files from the project.
 * Called by `npx convex ai-files remove`.
 *
 * - Strips the Convex section from AGENTS.md (deletes file if empty)
 * - Strips the Convex section from CLAUDE.md (deletes file if empty)
 * - Runs `npx skills remove <name...> --yes` for each tracked skill
 * - Deletes the `convex/_generated/ai/` directory
 */
export async function removeAiFiles(
  projectDir: string,
  convexDir: string,
): Promise<void> {
  const config = await readAiConfig(projectDir, convexDir);

  if (config === null) {
    logMessage("No Convex AI files found — nothing to remove.");
    return;
  }

  let removedCount = 0;

  // Strip Convex section from AGENTS.md.
  const stripped = await stripAgentsMdSection(projectDir);
  if (stripped) {
    logMessage(
      `${chalkStderr.green(`✔`)} Removed Convex section from AGENTS.md.`,
    );
    removedCount++;
  }

  // Strip Convex section from CLAUDE.md if present.
  // If the file is empty after stripping, it's deleted automatically.
  const strippedClaude = await stripClaudeMdSection(projectDir);
  if (strippedClaude === "section") {
    logMessage(
      `${chalkStderr.green(`✔`)} Removed Convex section from CLAUDE.md.`,
    );
    removedCount++;
  } else if (strippedClaude === "file") {
    logMessage(`${chalkStderr.green(`✔`)} Deleted CLAUDE.md.`);
    removedCount++;
  }

  // Remove installed skills via the skills CLI.
  if (config.installedSkillNames.length > 0) {
    if (await shouldRunSkillsCli()) {
      logMessage(
        `Removing Convex agent skills: ${config.installedSkillNames.join(", ")}`,
      );
      const skillsOk = await runSkillsRemove(
        projectDir,
        config.installedSkillNames,
      );
      if (!skillsOk) {
        logMessage(
          chalkStderr.yellow(
            "Could not remove agent skills automatically. Remove them manually with: npx skills remove",
          ),
        );
      } else {
        const removedLock = await removeSkillsLockIfEmpty(
          projectDir,
          config.installedSkillNames,
        );
        if (removedLock) {
          logMessage(`${chalkStderr.green(`✔`)} Deleted skills-lock.json.`);
          removedCount++;
        }
      }
    }
  }

  // Clean up the legacy `.cursor/rules/convex_rules.mdc` file if present.
  const removedLegacy = await removeLegacyCursorRulesFile(projectDir);
  if (removedLegacy) {
    logMessage(
      `${chalkStderr.green(`✔`)} Removed legacy .cursor/rules/convex_rules.mdc.`,
    );
    removedCount++;
  }

  // Delete the convex/_generated/ai/ directory.
  try {
    await fs.rm(aiDirForConvexDir(convexDir), { recursive: true, force: true });
    logMessage(
      `${chalkStderr.green(`✔`)} Deleted ${path.relative(projectDir, aiDirForConvexDir(convexDir))}/`,
    );
    removedCount++;
  } catch (error) {
    Sentry.captureException(error);
    logMessage(
      chalkStderr.yellow(
        `Could not delete ${path.relative(projectDir, aiDirForConvexDir(convexDir))}/. Remove it manually.`,
      ),
    );
  }

  if (removedCount > 0) {
    logMessage("Convex AI files removed.");
  }
}

/**
 * Called by `npx convex ai-files disable`.
 *
 * Writes a suppression flag into `convex.json` (`aiFiles.disableStalenessMessage`) so
 * `npx convex dev` stops showing AI files install/staleness messages.
 * Files are left in place - use `remove` to delete them.
 * The user can re-enable at any time with `npx convex ai-files enable`.
 */
export async function disableAiFiles(projectDir: string): Promise<void> {
  try {
    await writeAiDisabledToProjectConfig(true, projectDir);
    logMessage(
      `${chalkStderr.green(`✔`)} Convex AI file staleness/install messages disabled. Run ${chalkStderr.bold(`npx convex ai-files enable`)} to re-enable.`,
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

/**
 * Print the current status of Convex AI files to the terminal.
 */
export async function statusAiFiles(
  projectDir: string,
  convexDir: string,
): Promise<void> {
  const convexDirName = path.relative(projectDir, convexDir);
  const guidelinesRelPath = path.relative(
    projectDir,
    guidelinesPathForConvexDir(convexDir),
  );

  const config = await readAiConfig(projectDir, convexDir);

  if (config === null) {
    logMessage(`Convex AI files: ${chalkStderr.yellow("not installed")}`);
    logMessage(
      `  Run ${chalkStderr.bold("npx convex ai-files install")} to get started, ` +
        `or ${chalkStderr.bold("npx convex ai-files disable")} to silence this message.`,
    );
    return;
  }

  if (config.disableStalenessMessage) {
    logMessage(
      `Convex AI files: ${chalkStderr.yellow("staleness/install messages disabled")}`,
    );
    logMessage(
      `  Run ${chalkStderr.bold("npx convex ai-files enable")} to re-enable.`,
    );
    return;
  }

  logMessage(`Convex AI files: ${chalkStderr.green("enabled")}`);

  // Fetch canonical version data in parallel with the file reads below
  // (best-effort — network may be unavailable in CI or offline envs).
  const [versionData, guidelinesFile, agentsContent, claudeContent] =
    await Promise.all([
      getVersion(),
      readFileSafe(guidelinesPathForConvexDir(convexDir)),
      readFileSafe(agentsMdPath(projectDir)),
      readFileSafe(claudeMdPath(projectDir)),
    ]);

  const canonicalGuidelinesHash = versionData?.guidelinesHash ?? null;
  const canonicalAgentSkillsSha = versionData?.agentSkillsSha ?? null;
  const networkAvailable = versionData !== null;

  // --- convex/_generated/ai/guidelines.md ---
  if (guidelinesFile === null) {
    logMessage(
      `  ${chalkStderr.yellow("⚠")} ${guidelinesRelPath}: not on disk — run ${chalkStderr.bold("npx convex ai-files install")} to reinstall`,
    );
  } else if (
    config.guidelinesHash !== null &&
    hashSha256(guidelinesFile) !== config.guidelinesHash
  ) {
    logMessage(
      `  ${chalkStderr.yellow("⚠")} ${guidelinesRelPath}: installed, modified locally (Convex updates will be skipped)`,
    );
  } else if (
    networkAvailable &&
    canonicalGuidelinesHash !== null &&
    config.guidelinesHash !== null &&
    config.guidelinesHash !== canonicalGuidelinesHash
  ) {
    logMessage(
      `  ${chalkStderr.yellow("⚠")} ${guidelinesRelPath}: installed, out of date — run ${chalkStderr.bold("npx convex ai-files update")}`,
    );
  } else {
    logMessage(
      `  ${chalkStderr.green("✔")} ${guidelinesRelPath}: installed${networkAvailable ? ", up to date" : ""}`,
    );
  }

  // --- AGENTS.md (Convex section) ---
  const hasAgentsSection =
    agentsContent !== null &&
    agentsContent.includes(AGENTS_MD_START_MARKER) &&
    agentsContent.includes(AGENTS_MD_END_MARKER);

  if (!hasAgentsSection) {
    logMessage(
      `  ${chalkStderr.yellow("⚠")} AGENTS.md: Convex section missing — run ${chalkStderr.bold("npx convex ai-files install")} to reinstall`,
    );
  } else {
    // Staleness is locally computable from the template — no network needed.
    const currentSectionHash = hashSha256(agentsMdConvexSection(convexDirName));
    if (
      config.agentsMdSectionHash !== null &&
      config.agentsMdSectionHash !== currentSectionHash
    ) {
      logMessage(
        `  ${chalkStderr.yellow("⚠")} AGENTS.md: Convex section out of date — run ${chalkStderr.bold("npx convex ai-files update")}`,
      );
    } else {
      logMessage(
        `  ${chalkStderr.green("✔")} AGENTS.md: Convex section present, up to date`,
      );
    }
  }

  // --- CLAUDE.md (Convex section) ---
  const hasClaudeSection =
    claudeContent !== null &&
    claudeContent.includes(CLAUDE_MD_START_MARKER) &&
    claudeContent.includes(CLAUDE_MD_END_MARKER);

  if (!hasClaudeSection) {
    if (claudeContent === null) {
      logMessage(
        `  ${chalkStderr.yellow("⚠")} CLAUDE.md: missing - run ${chalkStderr.bold("npx convex ai-files install")} to create it`,
      );
    } else {
      logMessage(
        `  ${chalkStderr.yellow("⚠")} CLAUDE.md: no Convex section present - run ${chalkStderr.bold("npx convex ai-files update")} to add it`,
      );
    }
  } else {
    // Staleness is locally computable from the template — no network needed.
    const currentSectionHash = hashSha256(claudeMdConvexSection(convexDirName));
    if (
      config.claudeMdHash !== null &&
      config.claudeMdHash !== currentSectionHash
    ) {
      logMessage(
        `  ${chalkStderr.yellow("⚠")} CLAUDE.md: Convex section out of date - run ${chalkStderr.bold("npx convex ai-files update")}`,
      );
    } else {
      logMessage(
        `  ${chalkStderr.green("✔")} CLAUDE.md: Convex section present, up to date`,
      );
    }
  }

  // --- Agent skills ---
  if (config.installedSkillNames.length === 0) {
    logMessage(
      `  ${chalkStderr.yellow("⚠")} Agent skills: not installed — run ${chalkStderr.bold("npx convex ai-files install")} to install`,
    );
  } else {
    const skillsStale =
      networkAvailable &&
      canonicalAgentSkillsSha !== null &&
      config.agentSkillsSha !== null &&
      config.agentSkillsSha !== canonicalAgentSkillsSha;
    const skillsList = config.installedSkillNames.join(", ");
    if (skillsStale) {
      logMessage(
        `  ${chalkStderr.yellow("⚠")} Agent skills: ${skillsList} — out of date, run ${chalkStderr.bold("npx convex ai-files update")}`,
      );
    } else {
      logMessage(
        `  ${chalkStderr.green("✔")} Agent skills: ${skillsList}${networkAvailable ? " (up to date)" : ""}`,
      );
    }
  }
}

export async function maybeSetupAiFiles(
  ctx: Context,
  convexDir: string,
  projectDir: string,
): Promise<void> {
  if (isAgentMode()) {
    return;
  }

  // Non-interactive (no TTY) is almost always an AI agent, not CI
  // (CI uses `npx convex deploy`). Default to installing so agents
  // get context automatically.
  let wantsAiFiles = true;
  if (process.stdin.isTTY) {
    wantsAiFiles = await promptYesNo(ctx, {
      message: "Set up Convex AI files? (guidelines, AGENTS.md, agent skills)",
      default: true,
    });
  }
  if (wantsAiFiles) {
    await writeAiFiles(convexDir, true, "quiet", projectDir);
  }
}

/**
 * Remove the legacy `.cursor/rules/convex_rules.mdc` file if it exists.
 * This file was written by the old cursor rules auto-update feature (removed
 * in favour of the AI files system). We clean it up unconditionally
 * during `writeAiFiles`, `convex ai-files update`, and `convex ai-files remove` since it was always
 * auto-managed and is now superseded by `convex/_generated/ai/guidelines.md`.
 */
async function removeLegacyCursorRulesFile(
  projectDir: string,
): Promise<boolean> {
  const filePath = path.join(
    projectDir,
    ".cursor",
    "rules",
    "convex_rules.mdc",
  );
  try {
    await fs.unlink(filePath);
    return true;
  } catch {
    return false;
  }
}

/**
 * Remove the skills-lock.json file if it only contains skills that we
 * are removing. The `npx skills remove` command leaves the lockfile behind
 * even when it's logically empty.
 */
async function removeSkillsLockIfEmpty(
  projectDir: string,
  removedSkillNames: string[],
): Promise<boolean> {
  const lockPath = path.join(projectDir, "skills-lock.json");
  try {
    const content = await fs.readFile(lockPath, "utf8");
    const lock = JSON.parse(content);

    // If the file doesn't match the expected structure, leave it alone.
    if (
      !lock ||
      typeof lock !== "object" ||
      !lock.skills ||
      typeof lock.skills !== "object"
    ) {
      return false;
    }

    const remainingSkills = Object.keys(lock.skills).filter(
      (name) => !removedSkillNames.includes(name),
    );

    if (remainingSkills.length === 0) {
      await fs.unlink(lockPath);
      return true;
    }
    return false;
  } catch {
    return false;
  }
}

async function readFileSafe(filePath: string): Promise<string | null> {
  try {
    return await fs.readFile(filePath, "utf8");
  } catch {
    return null;
  }
}

/**
 * Read the frontmatter `name:` values from skills installed by the skills CLI.
 */
async function readInstalledSkillNames(projectDir: string): Promise<string[]> {
  const skillsDir = path.join(projectDir, ".agents", "skills");
  let entries: string[];
  try {
    const dirents = await fs.readdir(skillsDir, { withFileTypes: true });
    entries = dirents
      .filter((d) => d.isDirectory() || d.isSymbolicLink())
      .map((d) => d.name);
  } catch {
    return [];
  }

  const names: string[] = [];
  for (const entry of entries) {
    const skillMdPath = path.join(skillsDir, entry, "SKILL.md");
    const content = await readFileSafe(skillMdPath);
    if (content === null) continue;
    // Extract `name: <value>` from YAML frontmatter (between the --- delimiters).
    const match = content.match(/^---[\s\S]*?^name:\s*(.+?)\s*$/m);
    if (match) {
      names.push(match[1]);
    }
  }
  return names;
}

/**
 * Runs `npx skills add get-convex/agent-skills --yes` in the given directory.
 * Output mode controls whether the skills CLI output is streamed live.
 * Returns true on success, false if the process fails or cannot be started.
 */
function runSkillsAdd(
  cwd: string,
  outputMode: "verbose" | "quiet" = "verbose",
): Promise<boolean> {
  return runSkillsCommand(
    cwd,
    ["add", "get-convex/agent-skills", "--yes"],
    outputMode,
  );
}

/**
 * Runs `npx skills remove <name...> --yes` to surgically remove only the
 * Convex-managed skills, leaving any skills from other sources intact.
 */
function runSkillsRemove(cwd: string, skillNames: string[]): Promise<boolean> {
  return runSkillsCommand(cwd, ["remove", ...skillNames, "--yes"]);
}

async function shouldRunSkillsCli(): Promise<boolean> {
  const versionData = await getVersion();
  if (versionData === null || versionData === undefined) {
    logMessage(chalkStderr.yellow(`Agent skills are temporarily disabled.`));
    return false;
  }
  if (versionData.disableSkillsCli) {
    logMessage(chalkStderr.yellow(`Agent skills are temporarily disabled.`));
    return false;
  }
  return true;
}

function runSkillsCommand(
  cwd: string,
  args: string[],
  outputMode: "verbose" | "quiet" = "verbose",
): Promise<boolean> {
  return new Promise((resolve) => {
    const quiet = outputMode === "quiet";
    const proc = child_process.spawn("npx", ["skills@latest", ...args], {
      cwd,
      stdio: quiet ? "pipe" : "inherit",
      // shell: true is required on Windows to resolve `npx` from PATH.
      shell: process.platform === "win32",
    });
    let capturedOutput = "";
    if (quiet) {
      proc.stdout?.on("data", (chunk) => {
        capturedOutput += chunk.toString();
      });
      proc.stderr?.on("data", (chunk) => {
        capturedOutput += chunk.toString();
      });
    }
    proc.on("close", (code) => {
      if (quiet && code !== 0 && capturedOutput.trim().length > 0) {
        const lines = capturedOutput.trim().split(/\r?\n/);
        const tail = lines.slice(-10).join("\n");
        logMessage(chalkStderr.gray(`skills output (tail):\n${tail}`));
      }
      resolve(code === 0);
    });
    proc.on("error", () => resolve(false));
  });
}
