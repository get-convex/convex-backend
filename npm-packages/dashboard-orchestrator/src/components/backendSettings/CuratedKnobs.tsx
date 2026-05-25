import type { KnobEntry } from "../../lib/orchestratorApi";
import { knobRowState } from "./knobOverrides";
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
        const rowState = knobRowState(knob, overrides, tierDefaults);
        return (
          <KnobRow
            key={knob.envVar}
            knob={knob}
            source={rowState.source}
            effectiveValue={rowState.effectiveValue}
            overrideValue={rowState.overrideValue}
            onOverride={(next) => onOverride(knob.envVar, next)}
            onReset={() => onReset(knob.envVar)}
          />
        );
      })}
    </div>
  );
}
