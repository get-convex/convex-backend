import type { KnobEntry } from "../../../../../lib/orchestratorApi";
import {
  hasExplicitOverride,
  knobRowState,
  visibleOverrideCount,
  type KnobRowSource,
} from "../../../../../components/backendSettings/knobOverrides";

export type AdvancedKnobShowFilter = "all" | "overridden" | "curated" | "tier";
export type { KnobRowSource };

export function filterAdvancedKnobs({
  registry,
  overrides,
  search,
  category,
  show,
}: {
  registry: KnobEntry[];
  overrides: Record<string, string>;
  search: string;
  category: string;
  show: AdvancedKnobShowFilter;
}): KnobEntry[] {
  const normalizedSearch = search.trim().toLowerCase();
  return registry.filter((knob) => {
    if (
      normalizedSearch &&
      ![
        knob.envVar,
        knob.displayName ?? "",
        knob.description,
        knob.category,
      ].some((value) => value.toLowerCase().includes(normalizedSearch))
    ) {
      return false;
    }
    if (category !== "ALL" && knob.category !== category) {
      return false;
    }
    if (show === "curated" && knob.exposure !== "curated") {
      return false;
    }
    if (show === "tier" && knob.exposure !== "tierTuned") {
      return false;
    }
    if (show === "overridden" && !hasExplicitOverride(overrides, knob.envVar)) {
      return false;
    }
    return true;
  });
}

export function advancedKnobRowState(
  knob: KnobEntry,
  overrides: Record<string, string>,
  tierDefaults: Record<string, string>,
): {
  source: KnobRowSource;
  effectiveValue: string;
  overrideValue: string;
} {
  return knobRowState(knob, overrides, tierDefaults);
}

export { visibleOverrideCount };
