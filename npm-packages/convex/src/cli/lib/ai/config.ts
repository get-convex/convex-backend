import * as Sentry from "@sentry/node";
// Use raw fs (not ctx.fs) so these operations run asynchronously and don't
// interfere with the file-watcher used by `convex dev`.
// eslint-disable-next-line no-restricted-imports
import { promises as fs } from "fs";
import path from "path";
import { z } from "zod";
import { aiFilesStatePathForConvexDir } from "./paths.js";

const aiFilesStateSchema = z.object({
  guidelinesHash: z.string().nullable(),
  agentsMdSectionHash: z.string().nullable(),
  claudeMdHash: z.string().nullable(),
  // Commit SHA from get-convex/agent-skills that was current when skills were
  // last installed. Used to detect when newer skills are available.
  agentSkillsSha: z.string().nullable(),
  // Names of skills installed by `npx skills add`, used by `remove` to
  // only remove Convex-managed skills.
  installedSkillNames: z.array(z.string()).default([]),
});

const aiFilesProjectConfigSchema = z
  .object({
    aiFiles: z
      .object({
        disableStalenessMessage: z.boolean().default(false),
      })
      .default({ disableStalenessMessage: false }),
  })
  .passthrough();

export const aiFilesSchema = aiFilesStateSchema;

type AiFilesState = z.infer<typeof aiFilesStateSchema>;
export type AiFilesConfig = AiFilesState & {
  disableStalenessMessage: boolean;
};

const EMPTY_AI_STATE: AiFilesState = {
  guidelinesHash: null,
  agentsMdSectionHash: null,
  claudeMdHash: null,
  agentSkillsSha: null,
  installedSkillNames: [],
};

async function readAiDisabledFromProjectConfig(
  projectDir: string,
): Promise<boolean> {
  let raw: string;
  try {
    raw = await fs.readFile(path.join(projectDir, "convex.json"), "utf8");
  } catch {
    return false;
  }
  try {
    const parsed = aiFilesProjectConfigSchema.parse(JSON.parse(raw));
    return parsed.aiFiles.disableStalenessMessage;
  } catch (err) {
    Sentry.captureException(err);
    return false;
  }
}

async function writeAiDisabledToProjectConfig(
  disableStalenessMessage: boolean,
  projectDir: string,
): Promise<void> {
  const filePath = path.join(projectDir, "convex.json");
  let existing: unknown = {};
  try {
    existing = JSON.parse(await fs.readFile(filePath, "utf8"));
  } catch {
    // Use a minimal object when convex.json doesn't exist yet or is unreadable.
  }
  const base =
    existing !== null &&
    typeof existing === "object" &&
    !Array.isArray(existing)
      ? (existing as Record<string, unknown>)
      : {};
  const aiFilesValue =
    base.aiFiles !== null &&
    typeof base.aiFiles === "object" &&
    !Array.isArray(base.aiFiles)
      ? (base.aiFiles as Record<string, unknown>)
      : {};
  const { $schema, ...rest } = base;
  const next: Record<string, unknown> = {
    $schema: $schema ?? "node_modules/convex/schemas/convex.schema.json",
    ...rest,
    aiFiles: {
      ...aiFilesValue,
      disableStalenessMessage,
    },
  };
  await fs.writeFile(filePath, JSON.stringify(next, null, 2) + "\n", "utf8");
}

export async function readAiConfig(
  projectDir: string,
  convexDir: string,
): Promise<AiFilesConfig | null> {
  const disableStalenessMessage =
    await readAiDisabledFromProjectConfig(projectDir);
  let rawState: string;
  try {
    rawState = await fs.readFile(
      aiFilesStatePathForConvexDir(convexDir),
      "utf8",
    );
  } catch {
    // State file doesn't exist yet — only treat as configured if user explicitly
    // disabled AI files in convex.json.
    return disableStalenessMessage
      ? { ...EMPTY_AI_STATE, disableStalenessMessage }
      : null;
  }
  try {
    const state = aiFilesStateSchema.parse(JSON.parse(rawState));
    return {
      ...state,
      disableStalenessMessage,
    };
  } catch (err) {
    Sentry.captureException(err);
    return null;
  }
}

export async function writeAiConfig(
  config: AiFilesConfig,
  projectDir: string,
  convexDir: string,
  options?: { persistDisabledPreference?: "ifTrue" | "always" | "never" },
): Promise<void> {
  const state = aiFilesStateSchema.parse({
    guidelinesHash: config.guidelinesHash,
    agentsMdSectionHash: config.agentsMdSectionHash,
    claudeMdHash: config.claudeMdHash,
    agentSkillsSha: config.agentSkillsSha,
    installedSkillNames: config.installedSkillNames,
  });
  await fs.writeFile(
    aiFilesStatePathForConvexDir(convexDir),
    JSON.stringify(state, null, 2) + "\n",
    "utf8",
  );
  const persistMode = options?.persistDisabledPreference ?? "ifTrue";
  if (
    persistMode === "always" ||
    (persistMode === "ifTrue" && config.disableStalenessMessage)
  ) {
    await writeAiDisabledToProjectConfig(
      config.disableStalenessMessage,
      projectDir,
    );
  }
}
