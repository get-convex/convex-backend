import { useCallback, useMemo } from "react";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { useBBMutation, useBBQuery, useMutate } from "./api";

const OFFERS_PATH = "/member/directory_sync_offers";
const DIRECTORY_SYNC_PATH = "/teams/{team_id}/directory_sync";
const GROUPS_PATH = "/teams/{team_id}/directory_sync/groups";
const MAPPING_PATH =
  "/teams/{team_id}/directory_sync/mappings/{workos_group_id}";

export const DIRECTORY_GROUPS_PAGE_SIZE = 25;

export function useGetDirectorySync(
  teamId: number | undefined,
  { isPaused = false }: { isPaused?: boolean } = {},
) {
  const { data, isLoading } = useBBQuery({
    path: DIRECTORY_SYNC_PATH,
    pathParams: {
      team_id: isPaused ? "" : (teamId?.toString() ?? ""),
    },
  });
  return { data, isLoading };
}

export function useGenerateDirectorySyncConfigurationLink(teamId: number) {
  return useBBMutation({
    path: "/teams/{team_id}/directory_sync/portal_link",
    pathParams: {
      team_id: teamId.toString(),
    },
  });
}

export function useDisableDirectorySync(teamId: number) {
  return useBBMutation({
    path: "/teams/{team_id}/directory_sync/disable",
    pathParams: {
      team_id: teamId.toString(),
    },
    mutateKey: DIRECTORY_SYNC_PATH,
    mutatePathParams: {
      team_id: teamId.toString(),
    },
    successToast: "Directory Sync has been disabled for your team.",
  });
}

export function useDirectorySyncGroups(
  teamId: number | undefined,
  cursor: string | undefined,
  { isPaused = false }: { isPaused?: boolean } = {},
) {
  const queryParams = useMemo(
    () => ({ cursor, limit: DIRECTORY_GROUPS_PAGE_SIZE }),
    [cursor],
  );
  const { data, isLoading, error } = useBBQuery({
    path: GROUPS_PATH,
    pathParams: {
      team_id: isPaused ? "" : (teamId?.toString() ?? ""),
    },
    queryParams,
  });
  return { data, isLoading, error };
}

export function useSetGroupRoleMapping(teamId: number, workosGroupId: string) {
  return useBBMutation({
    method: "put",
    path: MAPPING_PATH,
    pathParams: {
      team_id: teamId,
      workos_group_id: workosGroupId,
    },
    mutateKey: GROUPS_PATH,
    mutatePathParams: {
      team_id: teamId.toString(),
    },
    successToast: "Group role updated.",
  });
}

// Teams whose directory roster lists one of the caller's verified emails and
// that they aren't a member of yet, so they can join without an invitation.
// Undefined until directory sync is rolled out to the member.
export function useDirectorySyncOffers() {
  const { directorySync } = useLaunchDarkly();
  const { data } = useBBQuery({
    path: OFFERS_PATH,
    pathParams: undefined,
    swrOptions: {
      isPaused: () => !directorySync,
      // The roster behind these offers changes rarely, and they only surface
      // in the team switcher, so don't chase window focus and collapse the
      // repeat mounts into at most one request a minute.
      revalidateOnFocus: false,
      dedupingInterval: 1000 * 60,
    },
  });
  // Pausing stops new requests but leaves whatever SWR already cached, so the
  // flag has to gate the result too — otherwise turning it off mid-session
  // keeps offering teams the member should no longer be able to join.
  return directorySync ? data?.offers : undefined;
}

export function useJoinDirectorySyncedTeam() {
  const join = useBBMutation({
    path: "/member/directory_sync/join",
    pathParams: undefined,
    mutateKey: "/teams",
    successToast: "Joined team.",
  });
  const mutate = useMutate();
  return useCallback(
    async (body: { proposedTeamId: number }) => {
      const result = await join(body);
      // `mutateKey` only covers the teams list; the accepted offer also has to
      // disappear from the offers list.
      await mutate([OFFERS_PATH]);
      return result;
    },
    [join, mutate],
  );
}
