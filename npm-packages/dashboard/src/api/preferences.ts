import { MemberPreferences } from "generatedApi";
import { useBBMutation, useBBQuery } from "./api";

export function useMemberPreferences(): MemberPreferences | undefined {
  const { data } = useBBQuery({
    path: "/preferences",
    pathParams: undefined,
    swrOptions: {
      revalidateOnMount: false,
      revalidateIfStale: false,
      revalidateOnFocus: false,
      revalidateOnReconnect: false,
    },
  });
  return data?.preferences;
}

export function useSetPreference() {
  return useBBMutation({
    method: "put",
    path: "/set_preference",
    pathParams: undefined,
    mutateKey: "/preferences",
  });
}
