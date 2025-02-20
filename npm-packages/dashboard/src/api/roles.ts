import { useCurrentTeam, useTeamMembers } from "api/teams";
import { useBBMutation, useBBQuery } from "./api";
import { useProfile } from "./profile";
import { useCurrentProject } from "./projects";

export function useIsCurrentMemberTeamAdmin(): boolean {
  const team = useCurrentTeam();
  const profile = useProfile();
  const members = useTeamMembers(team?.id);
  const member = members?.find((m) => m.id === profile?.id);
  return member?.role === "admin";
}

export function useHasProjectAdminPermissions(projectId?: number): boolean {
  const isTeamAdmin = useIsCurrentMemberTeamAdmin();
  const { projectRoles } = useProjectRoles();
  const filteredProjectRoles = projectRoles?.filter(
    (r) => r.projectId === projectId,
  );
  const profile = useProfile();
  if (isTeamAdmin) {
    return true;
  }

  return (
    filteredProjectRoles?.some(
      (role) => role.role === "admin" && role.memberId === profile?.id,
    ) ?? false
  );
}

export function useHasProjectAdminPermissionsForProject(): (
  projectId: number,
) => boolean {
  const isTeamAdmin = useIsCurrentMemberTeamAdmin();
  const { projectRoles } = useProjectRoles();
  const profile = useProfile();
  if (isTeamAdmin) {
    return () => true;
  }

  return (projectId: number) => {
    const filteredProjectRoles = projectRoles?.filter(
      (r) => r.projectId === projectId,
    );
    return (
      filteredProjectRoles?.some(
        (role) => role.role === "admin" && role.memberId === profile?.id,
      ) ?? false
    );
  };
}

export function useProjectRoles() {
  const team = useCurrentTeam();
  const { data, isLoading } = useBBQuery({
    path: `/teams/{team_id}/get_project_roles`,
    pathParams: {
      team_id: team?.id.toString() || "",
    },
  });

  return { isLoading, projectRoles: data };
}

export function useCurrentProjectRoles() {
  const project = useCurrentProject();
  const projectRoles = useProjectRoles();
  return (
    project &&
    (projectRoles.projectRoles?.filter(
      (role) => role.projectId === project.id,
    ) ||
      [])
  );
}

export function useUpdateProjectRoles(teamId?: number) {
  return useBBMutation({
    path: `/teams/{team_id}/update_project_roles`,
    pathParams: {
      team_id: teamId?.toString() || "",
    },
    mutateKey: `/teams/{team_id}/get_project_roles`,
    mutatePathParams: {
      team_id: teamId?.toString() || "",
    },
    successToast: "Project roles updated.",
  });
}

export function useUpdateTeamMemberRole(teamId: number) {
  return useBBMutation({
    path: `/teams/{team_id}/update_member_role`,
    pathParams: {
      team_id: teamId.toString(),
    },
    mutateKey: `/teams/{team_id}/members`,
    successToast: "Member role updated.",
  });
}
