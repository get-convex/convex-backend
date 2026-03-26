import {
  CLAUDE_MD_END_MARKER,
  CLAUDE_MD_START_MARKER,
  claudeMdConvexSection,
} from "../../codegen_templates/claudemd.js";
import { claudeMdPath } from "./paths.js";
import { type AiFilesConfig } from "./config.js";
import {
  type ManagedSectionTarget,
  type InjectResult,
  type StripResult,
  injectManagedSection,
  stripManagedSection,
  hasManagedSection,
  removeMarkdownSection,
} from "./utils.js";

function target(projectDir?: string): ManagedSectionTarget {
  return {
    filePath: claudeMdPath(projectDir),
    startMarker: CLAUDE_MD_START_MARKER,
    endMarker: CLAUDE_MD_END_MARKER,
  };
}

export async function injectClaudeMdSection({
  section,
  projectDir,
}: {
  section: string;
  projectDir?: string;
}): Promise<InjectResult> {
  return injectManagedSection({ ...target(projectDir), section });
}

export async function stripClaudeMdSection(
  projectDir: string,
): Promise<StripResult> {
  return stripManagedSection(target(projectDir));
}

export async function removeClaudeMdSection(
  projectDir: string,
): Promise<boolean> {
  return removeMarkdownSection({
    projectDir,
    strip: stripClaudeMdSection,
    fileName: "CLAUDE.md",
  });
}

export async function hasClaudeMdInstalled(
  projectDir: string,
): Promise<boolean> {
  return hasManagedSection(target(projectDir));
}

/**
 * Inject (or update) the Convex section in CLAUDE.md and record the hash.
 * Returns true if the file was actually written.
 */
export async function applyClaudeMdSection({
  projectDir,
  config,
  convexDirName,
}: {
  projectDir: string;
  config: AiFilesConfig;
  convexDirName: string;
}): Promise<boolean> {
  const result = await injectClaudeMdSection({
    section: claudeMdConvexSection(convexDirName),
    projectDir,
  });
  config.claudeMdHash = result.sectionHash;
  return result.didWrite;
}
