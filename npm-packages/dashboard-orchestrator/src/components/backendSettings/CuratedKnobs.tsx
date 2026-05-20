import type { KnobEntry } from "../../lib/orchestratorApi";
import { KnobRow } from "./KnobRow";

export function CuratedKnobs({
  registry,
  tierDefaults,
  overrides,
  onOverride,
  onReset,
}: {
  registry: KnobEntry[];
  tierDefaults: Record<string, string>;
  overrides: Record<string, string>;
  onOverride: (envVar: string, next: string) => void;
  onReset: (envVar: string) => void;
}) {
  const curated = registry.filter((k) => k.exposure === "curated");
  return (
    <div className="flex flex-col divide-y">
      {curated.map((knob) => {
        const override = overrides[knob.envVar];
        const tierDefault = tierDefaults[knob.envVar];
        const source: "override" | "tier" | "default" = override
          ? "override"
          : tierDefault
            ? "tier"
            : "default";
        const effective = override ?? tierDefault ?? "";
        return (
          <KnobRow
            key={knob.envVar}
            knob={knob}
            source={source}
            effectiveValue={effective}
            overrideValue={override ?? ""}
            onOverride={(next) => onOverride(knob.envVar, next)}
            onReset={() => onReset(knob.envVar)}
          />
        );
      })}
    </div>
  );
}
