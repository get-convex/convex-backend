import {
  Disclosure,
  DisclosureButton,
  DisclosurePanel,
} from "@headlessui/react";
import { ChevronDownIcon, ChevronRightIcon } from "@radix-ui/react-icons";
import { useState } from "react";
import type { HostCapacity, KnobEntry } from "../../lib/orchestratorApi";
import { CapacityStrip } from "./CapacityStrip";
import { CuratedKnobs } from "./CuratedKnobs";
import { TierSelector } from "./TierSelector";

export type BackendSettingsDraft = {
  tier: string;
  overrides: Record<string, string>;
};

export function BackendSettingsForm({
  registry,
  capacity,
  tierDefaults,
  initial,
  onChange,
}: {
  registry: KnobEntry[] | undefined;
  capacity: HostCapacity | undefined;
  tierDefaults: Record<string, string>;
  initial: BackendSettingsDraft;
  onChange: (next: BackendSettingsDraft) => void;
}) {
  const [draft, setDraft] = useState(initial);

  const update = (next: BackendSettingsDraft) => {
    setDraft(next);
    onChange(next);
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-2">
        <span className="text-sm font-medium text-content-primary">Tier</span>
        <TierSelector
          value={draft.tier}
          capacity={capacity}
          onChange={(tier) => update({ ...draft, tier })}
        />
        <CapacityStrip capacity={capacity} selectedTier={draft.tier} />
      </div>
      <Disclosure>
        {({ open }) => (
          <>
            <DisclosureButton className="flex w-full items-center gap-1 text-left text-sm font-medium text-content-primary">
              {open ? <ChevronDownIcon /> : <ChevronRightIcon />}
              Backend settings ({Object.keys(draft.overrides).length}{" "}
              customized)
            </DisclosureButton>
            <DisclosurePanel className="pt-2">
              {!registry ? (
                <div className="text-xs text-content-secondary">
                  Loading knob registry…
                </div>
              ) : (
                <CuratedKnobs
                  registry={registry}
                  tierDefaults={tierDefaults}
                  overrides={draft.overrides}
                  onOverride={(envVar, next) =>
                    update({
                      ...draft,
                      overrides: { ...draft.overrides, [envVar]: next },
                    })
                  }
                  onReset={(envVar) => {
                    const { [envVar]: _, ...rest } = draft.overrides;
                    update({ ...draft, overrides: rest });
                  }}
                />
              )}
            </DisclosurePanel>
          </>
        )}
      </Disclosure>
    </div>
  );
}
