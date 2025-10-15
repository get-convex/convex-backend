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
  const { data: teams, isValidating } = useBBQuery({
    path: "/teams",
    pathParams: undefined,
    swrOptions: {
      revalidateOnMount: !initialData,
    },
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

export function useTeamMembers(teamId: number | undefined) {
  const { data: members } = useBBQuery({
    path: "/teams/{team_id}/members",
    pathParams: {
      team_id: teamId?.toString() || "",
    },
  });
  return members;
}

export function useTeamEntitlements(teamId: number | undefined) {
  const { data: entitlements } = useBBQuery({
    path: "/teams/{team_id}/get_entitlements",
    pathParams: {
      team_id: teamId?.toString() || "",
    },
  });
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

export function useGetSSO(teamId: number | undefined) {
  const { data: ssoOrganization } = useBBQuery({
    path: "/teams/{team_id}/get_sso",
    pathParams: {
      team_id: teamId?.toString() || "",
    },
  });
  return ssoOrganization;
}

export function useEnableSSO(teamId: number) {
  return useBBMutation({
    path: `/teams/{team_id}/enable_sso`,
    pathParams: {
      team_id: teamId.toString(),
    },
    mutateKey: "/teams/{team_id}/get_sso",
    mutatePathParams: {
      team_id: teamId.toString(),
    },
    successToast: "SSO has been enabled for your team.",
  });
}

export function useUpdateSSODomain(teamId: number) {
  return useBBMutation({
    path: `/teams/{team_id}/update_sso_domain`,
    pathParams: {
      team_id: teamId.toString(),
    },
    mutateKey: "/teams/{team_id}/get_sso",
    mutatePathParams: {
      team_id: teamId.toString(),
    },
    successToast: "SSO domain has been updated.",
  });
}

export function useDisableSSO(teamId: number) {
  return useBBMutation({
    path: `/teams/{team_id}/disable_sso`,
    pathParams: {
      team_id: teamId.toString(),
    },
    mutateKey: "/teams/{team_id}/get_sso",
    mutatePathParams: {
      team_id: teamId.toString(),
    },
    successToast: "SSO has been disabled for your team.",
  });
}
