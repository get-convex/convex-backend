import type { KnobEntry } from "../../lib/orchestratorApi";

export type KnobRowSource = "default" | "tier" | "override";

export function hasExplicitOverride(
  overrides: Record<string, string>,
  envVar: string,
): boolean {
  return Object.prototype.hasOwnProperty.call(overrides, envVar);
}

export function visibleOverrideCount(
  overrides: Record<string, string>,
  registry: KnobEntry[],
): number {
  const visibleEnvVars = new Set(registry.map((knob) => knob.envVar));
  return Object.keys(overrides).filter((envVar) => visibleEnvVars.has(envVar))
    .length;
}

export function clearVisibleOverrides(
  overrides: Record<string, string>,
  registry: KnobEntry[],
): Record<string, string> {
  const visibleEnvVars = new Set(registry.map((knob) => knob.envVar));
  return Object.fromEntries(
    Object.entries(overrides).filter(([envVar]) => !visibleEnvVars.has(envVar)),
  );
}

export function knobRowState(
  knob: KnobEntry,
  overrides: Record<string, string>,
  tierDefaults: Record<string, string>,
): {
  source: KnobRowSource;
  effectiveValue: string;
  overrideValue: string;
} {
  if (hasExplicitOverride(overrides, knob.envVar)) {
    const overrideValue = overrides[knob.envVar];
    return {
      source: "override",
      effectiveValue: overrideValue,
      overrideValue,
    };
  }

  const tierDefault = tierDefaults[knob.envVar];
  if (tierDefault !== undefined) {
    return {
      source: "tier",
      effectiveValue: tierDefault,
      overrideValue: "",
    };
  }

  return {
    source: "default",
    effectiveValue: knob.defaultValue ?? "",
    overrideValue: "",
  };
}
