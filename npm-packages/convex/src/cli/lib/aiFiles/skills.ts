import child_process from "child_process";
import path from "path";
// eslint-disable-next-line no-restricted-imports
import { promises as fs } from "fs";
import { chalkStderr } from "chalk";
import { logMessage } from "../../../bundler/log.js";
import { getVersion, fetchAgentSkillsSha } from "../versionApi.js";
import { type AiFilesConfig } from "./config.js";
import { iife, readFileSafe } from "./utils.js";

/**
 * Read the frontmatter `name:` values from skills installed by the skills CLI.
 */
async function readInstalledSkillNames(projectDir: string): Promise<string[]> {
  const skillsDir = path.join(projectDir, ".agents", "skills");
  const entries = await iife(async () => {
    try {
      const dirents = await fs.readdir(skillsDir, { withFileTypes: true });
      return dirents
        .filter((d) => d.isDirectory() || d.isSymbolicLink())
        .map((d) => d.name);
    } catch {
      return [] as string[];
    }
  });
  if (entries.length === 0) return [];

  const names: string[] = [];
  for (const entry of entries) {
    const skillMdPath = path.join(skillsDir, entry, "SKILL.md");
    const content = await readFileSafe(skillMdPath);
    if (content === null) continue;
    const match = content.match(/^---[\s\S]*?^name:\s*(.+?)\s*$/m);
    if (match) {
      names.push(match[1]);
    }
  }
  return names;
}

/**
 * Runs `npx skills add get-convex/agent-skills --yes` in the given directory.
 * Returns true on success, false if the process fails or cannot be started.
 */
function runSkillsAdd(cwd: string): Promise<boolean> {
  return runSkillsCommand(cwd, ["add", "get-convex/agent-skills", "--yes"]);
}

/**
 * Runs `npx skills remove <name...> --yes` to surgically remove only the
 * Convex-managed skills, leaving any skills from other sources intact.
 */
function runSkillsRemove({
  cwd,
  skillNames,
}: {
  cwd: string;
  skillNames: string[];
}): Promise<boolean> {
  return runSkillsCommand(cwd, ["remove", ...skillNames, "--yes"]);
}

/**
 * This function exists so we have a way to disable skills installs without pushing a new
 * version of the convex CLI
 */
async function shouldRunSkillsCli(): Promise<boolean> {
  const versionData = await getVersion();

  if (versionData.kind === "error") return true;

  if (versionData.data.disableSkillsCli) {
    logMessage(chalkStderr.yellow(`Agent skills are temporarily disabled.`));
    return false;
  }

  return true;
}

/**
 * Remove the skills-lock.json file if it only contains skills that we
 * are removing. The `npx skills remove` command leaves the lockfile behind
 * even when it's logically empty.
 */
async function removeSkillsLockIfEmpty({
  projectDir,
  removedSkillNames,
}: {
  projectDir: string;
  removedSkillNames: string[];
}): Promise<boolean> {
  const lockPath = path.join(projectDir, "skills-lock.json");
  try {
    const content = await fs.readFile(lockPath, "utf8");
    const lock = JSON.parse(content);

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

/**
 * Install Convex agent skills and record the SHA and names into the config.
 * Handles the kill-switch check and all logging internally.
 */
export async function installSkills({
  projectDir,
  config,
}: {
  projectDir: string;
  config: AiFilesConfig;
}): Promise<void> {
  if (!(await shouldRunSkillsCli())) return;

  logMessage("Installing Convex agent skills...");
  const skillsOk = await runSkillsAdd(projectDir);
  if (!skillsOk) {
    logMessage(
      chalkStderr.yellow(
        "Could not install agent skills. You can retry manually with: npx skills add get-convex/agent-skills",
      ),
    );
    return;
  }

  const sha = await fetchAgentSkillsSha();
  if (sha) config.agentSkillsSha = sha;

  const names = await readInstalledSkillNames(projectDir);
  if (names.length > 0) config.installedSkillNames = names;
}

/**
 * Remove Convex-managed agent skills and clean up the lock file if empty.
 * Returns true if any removal occurred.
 */
export async function removeInstalledSkills({
  projectDir,
  skillNames,
}: {
  projectDir: string;
  skillNames: string[];
}): Promise<boolean> {
  if (skillNames.length === 0 || !(await shouldRunSkillsCli())) return false;

  logMessage(`Removing Convex agent skills: ${skillNames.join(", ")}`);
  const skillsOk = await runSkillsRemove({ cwd: projectDir, skillNames });
  if (!skillsOk) {
    logMessage(
      chalkStderr.yellow(
        "Could not remove agent skills automatically. Remove them manually with: npx skills remove",
      ),
    );
    return false;
  }

  const lockRemoved = await removeSkillsLockIfEmpty({
    projectDir,
    removedSkillNames: skillNames,
  });
  if (lockRemoved)
    logMessage(`${chalkStderr.green("✔")} Deleted skills-lock.json.`);
  return true;
}

function runSkillsCommand(cwd: string, args: string[]): Promise<boolean> {
  return new Promise((resolve) => {
    const proc = child_process.spawn(
      "npx",
      ["--yes", "skills@latest", ...args],
      {
        cwd,
        stdio: "pipe",
        // .cmd files on Windows require shell execution.
        shell: process.platform === "win32",
      },
    );
    let capturedOutput = "";
    proc.stdout?.on("data", (chunk) => {
      capturedOutput += chunk.toString();
    });
    proc.stderr?.on("data", (chunk) => {
      capturedOutput += chunk.toString();
    });
    proc.on("close", (code) => {
      if (code !== 0 && capturedOutput.trim().length > 0) {
        const lines = capturedOutput.trim().split(/\r?\n/);
        const tail = lines.slice(-10).join("\n");
        logMessage(chalkStderr.gray(`skills output (tail):\n${tail}`));
      }
      resolve(code === 0);
    });
    proc.on("error", () => resolve(false));
  });
}
