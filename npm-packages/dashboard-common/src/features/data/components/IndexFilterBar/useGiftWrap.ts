import { useCallback, useContext } from "react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

export function useGiftWrap() {
  const { giftWrapDataFilters, useMemberPreference } = useContext(
    DeploymentInfoContext,
  );
  const { value: openedPreference, set } = useMemberPreference(
    "new_data_filters_opened",
  );
  const enabled = !!giftWrapDataFilters;

  const open = useCallback(() => {
    if (openedPreference !== true) void set(true);
  }, [openedPreference, set]);

  const rewrap = useCallback(() => {
    if (openedPreference !== false) void set(false);
  }, [openedPreference, set]);

  return {
    enabled,
    opened: enabled ? openedPreference : true,
    open,
    rewrap,
  };
}
