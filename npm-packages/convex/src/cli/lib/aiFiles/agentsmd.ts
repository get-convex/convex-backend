import {
  AGENTS_MD_START_MARKER,
  AGENTS_MD_END_MARKER,
  agentsMdConvexSection,
} from "../../codegen_templates/agentsmd.js";
import { agentsMdPath } from "./paths.js";
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
    filePath: agentsMdPath(projectDir),
    startMarker: AGENTS_MD_START_MARKER,
    endMarker: AGENTS_MD_END_MARKER,
  };
}

export async function injectAgentsMdSection({
  section,
  projectDir,
}: {
  section: string;
  projectDir?: string;
}): Promise<InjectResult> {
  return injectManagedSection({ ...target(projectDir), section });
}

export async function stripAgentsMdSection(
  projectDir: string,
): Promise<StripResult> {
  return stripManagedSection(target(projectDir));
}

export async function removeAgentsMdSection(
  projectDir: string,
): Promise<boolean> {
  return removeMarkdownSection({
    projectDir,
    strip: stripAgentsMdSection,
    fileName: "AGENTS.md",
  });
}

export async function hasAgentsMdInstalled(
  projectDir: string,
): Promise<boolean> {
  return hasManagedSection(target(projectDir));
}

/**
 * Inject (or update) the Convex section in AGENTS.md and record the hash.
 * Returns true if the file was actually written.
 */
export async function applyAgentsMdSection({
  projectDir,
  config,
  convexDirName,
}: {
  projectDir: string;
  config: AiFilesConfig;
  convexDirName: string;
}): Promise<boolean> {
  const result = await injectAgentsMdSection({
    section: agentsMdConvexSection(convexDirName),
    projectDir,
  });
  config.agentsMdSectionHash = result.sectionHash;
  return result.didWrite;
}
