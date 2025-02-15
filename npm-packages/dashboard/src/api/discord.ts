import { useBBMutation, useBBQuery } from "./api";

export function useDiscordAccounts() {
  const { data } = useBBQuery({
    path: "/discord/accounts",
    pathParams: undefined,
  });
  return data?.accounts;
}

export function useDiscordAuthorize() {
  return useBBMutation({
    path: "/discord/authorize",
    pathParams: undefined,
    successToast: "Your Convex account is now linked to your Discord account.",
  });
}

export function useUnlinkDiscordAccount() {
  return useBBMutation({
    path: "/discord/unlink",
    pathParams: undefined,
    mutateKey: "/discord/accounts",
    successToast: "Discord account unlinked.",
  });
}
