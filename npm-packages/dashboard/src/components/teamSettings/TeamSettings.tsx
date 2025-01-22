import {
  useTeams,
  useTeamMembers,
  useDeleteTeam,
  useUpdateTeam,
} from "api/teams";
import { useProjects } from "api/projects";
import { useTeamOrbSubscription } from "api/billing";
import { useIsCurrentMemberTeamAdmin } from "api/roles";
import { Team } from "generatedApi";
import { Sheet, Button } from "dashboard-common";
import { ConfirmationDialog } from "elements/ConfirmationDialog";
import { useState } from "react";
import { TeamForm } from "./TeamForm";

export function TeamSettings({ team }: { team: Team }) {
  const updateTeam = useUpdateTeam(team.id);
  const hasAdminPermissions = useIsCurrentMemberTeamAdmin();
  const { teams } = useTeams();
  const teamMembers = useTeamMembers(team.id);
  const projects = useProjects(team.id);
  const [showDeleteTeamModal, setShowDeleteTeamModal] = useState(false);
  const deleteTeam = useDeleteTeam(team.id);
  const { subscription } = useTeamOrbSubscription(team.id);
  return (
    <>
      <h2>Team Settings</h2>
      <TeamForm
        team={team}
        onUpdateTeam={updateTeam}
        hasAdminPermissions={hasAdminPermissions}
      />
      <Sheet>
        <h3 className="mb-4">Delete Team</h3>
        <p className="mb-4">
          Permanently delete this team. To delete your team, you must first
          remove all team members and delete all projects associated with the
          team.
        </p>
        {subscription && (
          <p className="mb-4">
            Deleting your team will automatically cancel your{" "}
            <span className="font-semibold">{subscription.plan.name}</span>{" "}
            subscription.
          </p>
        )}
        <Button
          variant="danger"
          onClick={() => setShowDeleteTeamModal(true)}
          disabled={
            !hasAdminPermissions ||
            !teams ||
            teams.length === 1 ||
            !teamMembers ||
            teamMembers.length > 1 ||
            !projects ||
            projects.length > 0
          }
          tip={
            !hasAdminPermissions
              ? "You do not have permission to delete this team."
              : teams && teams.length === 1
                ? "You cannot delete your last team."
                : teamMembers && teamMembers.length > 1
                  ? "You must remove all other team members before deleting the team."
                  : projects && projects.length > 0
                    ? "You must delete all projects before deleting the team."
                    : undefined
          }
        >
          Delete Team
        </Button>
        {showDeleteTeamModal && (
          <ConfirmationDialog
            onClose={() => setShowDeleteTeamModal(false)}
            onConfirm={deleteTeam}
            validationText={team.slug}
            confirmText="Delete"
            dialogTitle="Delete Team"
            dialogBody="Are you sure you want to delete this team? This action cannot be undone."
          />
        )}
      </Sheet>
    </>
  );
}
