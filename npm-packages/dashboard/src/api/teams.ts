import { Team } from "generatedApi";
import { useLastViewedTeam } from "hooks/useLastViewed";
import { useInitialData } from "hooks/useServerSideData";
import { useRouter } from "next/router";
import { useBBMutation, useBBQuery } from "./api";

export function useTeams(): {
  selectedTeamSlug?: string;
  teams?: Team[];
} {
  const [initialData] = useInitialData();
  const { data: teams, isValidating } = useBBQuery("/teams", undefined, {
    // If initial data has been loaded via SSR, we don't need to load teams.
    revalidateOnMount: !initialData,
  });
  const [lastViewedTeam] = useLastViewedTeam();
  const router = useRouter();

  const defaultSelectedTeamSlug =
    teams &&
    (teams.some((team) => team.slug === lastViewedTeam)
      ? lastViewedTeam
      : teams[0]
        ? teams[0].slug
        : undefined);

  const selectedTeamSlug =
    typeof router.query.team === "string"
      ? router.query.team
      : defaultSelectedTeamSlug;

  if (
    teams &&
    typeof router.query.team === "string" &&
    !teams?.some((team) => team.slug === router.query.team) &&
    !isValidating
  ) {
    void router.push("/404");
  }

  return { selectedTeamSlug, teams };
}

export function useCurrentTeam() {
  const { selectedTeamSlug, teams } = useTeams();
  const currentTeam =
    teams?.find((team) => team.slug === selectedTeamSlug) ?? undefined;
  const router = useRouter();
  if (currentTeam?.suspended) {
    void router.push("/suspended");
    return;
  }
  return currentTeam;
}

export function useTeamMembers(teamId?: number) {
  const { data: members } = useBBQuery("/teams/{team_id}/members", {
    team_id: teamId?.toString() || "",
  });
  return members;
}

export function useTeamEntitlements(teamId: number | undefined) {
  const { data: entitlements } = useBBQuery(
    "/teams/{team_id}/get_entitlements",
    {
      team_id: teamId?.toString() || "",
    },
  );
  return entitlements;
}

export function useCreateTeam() {
  return useBBMutation({
    path: "/teams",
    pathParams: undefined,
    mutateKey: "/teams",
    successToast: "Team created.",
  });
}

export function useDeleteTeam(teamId: number) {
  return useBBMutation({
    path: "/teams/{team_id}/delete",
    pathParams: {
      team_id: teamId.toString(),
    },
    mutateKey: "/teams",
    successToast: "Team deleted.",
  });
}

export function useUpdateTeam(teamId: number) {
  return useBBMutation({
    path: `/teams/{team_id}`,
    pathParams: {
      team_id: teamId.toString(),
    },
    mutateKey: "/teams",
    successToast: "Team settings updated.",
  });
}

export function useRemoveTeamMember(teamId: number) {
  return useBBMutation({
    path: `/teams/{team_id}/remove_member`,
    pathParams: {
      team_id: teamId.toString(),
    },
    mutateKey: "/teams/{team_id}/members",
    mutatePathParams: {
      team_id: teamId.toString(),
    },
    successToast: "Member removed from team.",
  });
}

export function useUnpauseTeam(teamId: number) {
  return useBBMutation({
    path: `/teams/{team_id}/unpause`,
    pathParams: {
      team_id: teamId.toString(),
    },
    mutateKey: `/teams/{team_id}/usage/team_usage_state`,
    successToast: "Your team has been restored.",
  });
}
