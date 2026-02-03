import { useMemo } from "react";
import { Fieldset, Legend, RadioGroup } from "@headlessui/react";
import { Region, sortRegions } from "elements/Region";
import { DeploymentRegionMetadata } from "@convex-dev/platform/managementApi";

export function DefaultRegionSelector({
  value,
  onChange,
  regions,
  teamSlug,
  disabledDueToPermissions = false,
}: {
  value: string | null;
  onChange: (region: string | null) => void;
  regions: DeploymentRegionMetadata[] | undefined;
  teamSlug: string | undefined;
  disabledDueToPermissions?: boolean;
}) {
  const sortedRegions = useMemo(
    () => (regions ? sortRegions(regions) : undefined),
    [regions],
  );

  return (
    <div className="mb-6">
      <Fieldset>
        <Legend className="mb-1 text-sm text-content-primary">
          Region for New Deployments
        </Legend>
        <RadioGroup name="defaultRegion" value={value} onChange={onChange}>
          <div className="grid max-w-xl auto-rows-fr gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {sortedRegions ? (
              <>
                <Region
                  region={null}
                  teamSlug={teamSlug}
                  disabledDueToPermissions={disabledDueToPermissions}
                />
                {sortedRegions.map((region) => (
                  <Region
                    key={region.name}
                    region={region}
                    teamSlug={teamSlug}
                    disabledDueToPermissions={disabledDueToPermissions}
                  />
                ))}
              </>
            ) : (
              [1, 2, 3].map((i) => (
                <Region
                  key={i}
                  region={undefined}
                  teamSlug={teamSlug}
                  disabledDueToPermissions={disabledDueToPermissions}
                />
              ))
            )}
          </div>
        </RadioGroup>
      </Fieldset>
    </div>
  );
}
