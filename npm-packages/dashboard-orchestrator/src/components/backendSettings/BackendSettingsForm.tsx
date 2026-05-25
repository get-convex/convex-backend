import {
  Disclosure,
  DisclosureButton,
  DisclosurePanel,
} from "@headlessui/react";
import { ChevronDownIcon, ChevronRightIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { useState } from "react";
import type { HostCapacity, KnobEntry } from "../../lib/orchestratorApi";
import { BackendInfrastructureForm } from "./BackendInfrastructureForm";
import {
  DEFAULT_INFRASTRUCTURE,
  type BackendInfrastructureDraft,
} from "./backendInfrastructure";
import { CapacityStrip } from "./CapacityStrip";
import { CuratedKnobs } from "./CuratedKnobs";
import { clearVisibleOverrides, visibleOverrideCount } from "./knobOverrides";
import { TierSelector } from "./TierSelector";

export type BackendSettingsDraft = {
  tier: string;
  overrides: Record<string, string>;
  infrastructure?: BackendInfrastructureDraft;
};

export function BackendSettingsForm({
  registry,
  capacity,
  tierDefaults,
  currentTier,
  showInfrastructure = false,
  initial,
  onChange,
}: {
  registry: KnobEntry[] | undefined;
  capacity: HostCapacity | undefined;
  tierDefaults: Record<string, string>;
  /**
   * When set (deployment settings page), the capacity strip subtracts
   * this tier's slice from the host total before projecting the
   * selected tier — avoids double-counting the deployment being resized.
   * Leave undefined for the project-creation flow (new deployment).
   */
  currentTier?: string;
  showInfrastructure?: boolean;
  initial: BackendSettingsDraft;
  onChange: (next: BackendSettingsDraft) => void;
}) {
  const [draft, setDraft] = useState(initial);

  const update = (next: BackendSettingsDraft) => {
    setDraft(next);
    onChange(next);
  };
  const customizedCount = registry
    ? visibleOverrideCount(draft.overrides, registry)
    : 0;

  const revertVisibleOverrides = () => {
    if (!registry) return;
    update({
      ...draft,
      overrides: clearVisibleOverrides(draft.overrides, registry),
    });
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
        <CapacityStrip
          capacity={capacity}
          selectedTier={draft.tier}
          currentTier={currentTier}
        />
      </div>
      <Disclosure>
        {({ open }) => (
          <>
            <div className="flex flex-wrap items-center justify-between gap-2">
              <DisclosureButton className="flex items-center gap-1 text-left text-sm font-medium text-content-primary">
                {open ? <ChevronDownIcon /> : <ChevronRightIcon />}
                Backend settings ({customizedCount} customized)
              </DisclosureButton>
              {customizedCount > 0 && (
                <Button
                  variant="neutral"
                  size="xs"
                  onClick={revertVisibleOverrides}
                >
                  Revert knobs to defaults
                </Button>
              )}
            </div>
            <DisclosurePanel className="pt-2">
              {showInfrastructure && (
                <div className="mb-4 border-b border-border-transparent pb-4">
                  <BackendInfrastructureForm
                    value={draft.infrastructure ?? DEFAULT_INFRASTRUCTURE}
                    onChange={(infrastructure) =>
                      update({ ...draft, infrastructure })
                    }
                  />
                </div>
              )}
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
