import * as Sentry from "@sentry/node";
// Use raw fs (not ctx.fs) so these operations run asynchronously and don't
// interfere with the file-watcher used by `convex dev`.
// eslint-disable-next-line no-restricted-imports
import { promises as fs } from "fs";
import path from "path";
import { z } from "zod";
import { aiFilesStatePathForConvexDir } from "./paths.js";
import { iife, readFileSafe } from "./utils.js";

export const aiFilesStateSchema = z.object({
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
        // `enabled` is the canonical field. When present it takes full
        // precedence - `enabled: true` will re-enable even if the legacy
        // disableStalenessMessage field is still `true` in the file.
        enabled: z.boolean().optional(),
        // @deprecated - use `enabled` instead. Read for backward compat;
        // new writes always emit `enabled` and drop this key.
        disableStalenessMessage: z.boolean().optional(),
      })
      .default({}),
  })
  .passthrough();

type AiFilesState = z.infer<typeof aiFilesStateSchema>;
export type AiFilesConfig = AiFilesState & {
  enabled: boolean;
};

const EMPTY_AI_STATE: AiFilesState = {
  guidelinesHash: null,
  agentsMdSectionHash: null,
  claudeMdHash: null,
  agentSkillsSha: null,
  installedSkillNames: [],
};

async function readAiEnabledFromProjectConfig(
  projectDir: string,
): Promise<boolean> {
  const raw = await readFileSafe(path.join(projectDir, "convex.json"));
  if (raw === null) return true;
  try {
    const parsed = aiFilesProjectConfigSchema.parse(JSON.parse(raw));
    // `enabled` takes full precedence when explicitly set.
    if (parsed.aiFiles.enabled !== undefined) return parsed.aiFiles.enabled;
    // Legacy `disableStalenessMessage` - invert it.
    return !(parsed.aiFiles.disableStalenessMessage ?? false);
  } catch (err) {
    Sentry.captureException(err);
    return true;
  }
}

export async function writeAiEnabledToProjectConfig({
  projectDir,
  enabled,
}: {
  projectDir: string;
  enabled: boolean;
}): Promise<void> {
  const filePath = path.join(projectDir, "convex.json");
  const existing = await iife(async () => {
    try {
      return JSON.parse(await fs.readFile(filePath, "utf8")) as unknown;
    } catch {
      return {} as unknown;
    }
  });
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
  // Remove legacy keys on every write.
  const { disableStalenessMessage: _legacy, ...restAiFiles } = aiFilesValue;
  const next: Record<string, unknown> = {
    $schema: $schema ?? "node_modules/convex/schemas/convex.schema.json",
    ...rest,
    aiFiles: { ...restAiFiles, enabled },
  };
  await fs.writeFile(filePath, JSON.stringify(next, null, 2) + "\n", "utf8");
}

export async function readAiConfig({
  projectDir,
  convexDir,
}: {
  projectDir: string;
  convexDir: string;
}): Promise<AiFilesConfig | null> {
  const enabled = await readAiEnabledFromProjectConfig(projectDir);
  const rawState = await readFileSafe(aiFilesStatePathForConvexDir(convexDir));
  if (rawState === null) {
    // No state file means AI files are not installed, unless the user has
    // explicitly disabled in convex.json.
    return !enabled ? { ...EMPTY_AI_STATE, enabled } : null;
  }
  try {
    const state = aiFilesStateSchema.parse(JSON.parse(rawState));
    return { ...state, enabled };
  } catch (err) {
    Sentry.captureException(err);
    return null;
  }
}

export async function hasAiFilesConfig({
  projectDir,
  convexDir,
}: {
  projectDir: string;
  convexDir: string;
}): Promise<boolean> {
  if (!(await readAiEnabledFromProjectConfig(projectDir))) {
    return true;
  }
  try {
    const rawState = await fs.readFile(
      aiFilesStatePathForConvexDir(convexDir),
      "utf8",
    );
    aiFilesStateSchema.parse(JSON.parse(rawState));
    return true;
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code !== "ENOENT") {
      Sentry.captureException(err);
    }
    return false;
  }
}

export async function writeAiConfig({
  config,
  projectDir,
  convexDir,
  options,
}: {
  config: AiFilesConfig;
  projectDir: string;
  convexDir: string;
  options?: { persistEnabledPreference?: "ifFalse" | "always" | "never" };
}): Promise<void> {
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

  const persistMode = options?.persistEnabledPreference ?? "ifFalse";
  if (
    persistMode === "always" ||
    (persistMode === "ifFalse" && !config.enabled)
  )
    await writeAiEnabledToProjectConfig({
      projectDir,
      enabled: config.enabled,
    });
}
