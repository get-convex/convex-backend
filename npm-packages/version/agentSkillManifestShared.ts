import { v } from "convex/values";
import { z } from "zod";

export const agentSkillEntrySchema = z.object({
  skillName: z.string(),
  directoryName: z.string(),
  skillHash: z.string(),
});

export const agentSkillManifestRequestSchema = z.object({
  repoSha: z.string(),
  skills: z.array(agentSkillEntrySchema),
});

export const agentSkillEntry = v.object({
  skillName: v.string(),
  directoryName: v.string(),
  skillHash: v.string(),
});

export type AgentSkill = z.infer<typeof agentSkillEntrySchema>;
export type AgentSkillManifestRequest = z.infer<
  typeof agentSkillManifestRequestSchema
>;

export function compareSkills(a: AgentSkill, b: AgentSkill) {
  return a.skillName.localeCompare(b.skillName);
}

export function findDuplicateSkillName({ skills }: { skills: AgentSkill[] }) {
  const seenSkillNames = new Set<string>();
  for (const { skillName } of skills) {
    if (seenSkillNames.has(skillName)) return skillName;
    seenSkillNames.add(skillName);
  }
  return null;
}

export function formatDuplicateSkillNameError({
  skillName,
}: {
  skillName: string;
}) {
  return `Duplicate skill name '${skillName}' in manifest`;
}

export function toCanonicalSkills({ skills }: { skills: AgentSkill[] }) {
  return skills.map(({ skillName, directoryName, skillHash }) => [
    skillName,
    directoryName,
    skillHash,
  ]);
}
