import { useContext } from "react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

/**
 * Whether to show the rebuilt index filter bar in place of the Filter & Sort
 * panel: the `newDataFilters` flag has to be on, and the member must not have
 * switched themselves back.
 */
export function useNewDataFilters() {
  const { newDataFilters, useMemberPreference } = useContext(
    DeploymentInfoContext,
  );
  const { value: optedOut, set: setOptedOut } = useMemberPreference(
    "new_data_filters_optout",
  );
  return {
    newDataFilters: !!newDataFilters && !optedOut,
    setOptedOut,
  };
}
