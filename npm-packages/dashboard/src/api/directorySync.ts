import { useCallback } from "react";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { useBBMutation, useBBQuery, useMutate } from "./api";

const OFFERS_PATH = "/member/directory_sync_offers";

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
