import { useCallback, useContext } from "react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

export function useGiftWrap() {
  const { useMemberPreference } = useContext(DeploymentInfoContext);
  const { value: opened, set } = useMemberPreference("new_data_filters_opened");

  const open = useCallback(() => {
    if (opened !== true) void set(true);
  }, [opened, set]);

  const rewrap = useCallback(() => {
    if (opened !== false) void set(false);
  }, [opened, set]);

  return { opened, open, rewrap };
}
