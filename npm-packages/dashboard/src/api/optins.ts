import { OptInToAccept } from "generatedApi";
import { useBBMutation, useBBQuery } from "./api";

export function useHasOptedIn(): {
  hasOptedIn?: boolean;
  isLoading?: boolean;
  optInsWithMessageToAccept?: OptInToAccept[];
} {
  const { data, isLoading } = useBBQuery("/optins", undefined, {
    refreshInterval: 0,
  });
  if (!data || isLoading) return { isLoading: true };
  if (!data.optInsToAccept) {
    throw new Error("Cannot determine optins status");
  }

  return {
    hasOptedIn: data.optInsToAccept.length === 0,
    optInsWithMessageToAccept: data.optInsToAccept,
  };
}

export function useAcceptOptIns() {
  return useBBMutation({
    method: "put",
    path: "/optins",
    pathParams: undefined,
    mutateKey: "/optins",
    toastOnError: false,
  });
}
