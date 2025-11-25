import {
  useTeams,
  useTeamMembers,
  useDeleteTeam,
  useUpdateTeam,
} from "api/teams";
import { useProjects } from "api/projects";
import { useTeamOrbSubscription } from "api/billing";
import { useIsCurrentMemberTeamAdmin } from "api/roles";
import { TeamResponse } from "generatedApi";
import { Sheet } from "@ui/Sheet";
import { Button } from "@ui/Button";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { useCallback, useState } from "react";
import startCase from "lodash/startCase";
import { OpenInVercel } from "components/OpenInVercel";
import { TeamForm } from "./TeamForm";

export function TeamSettings({ team }: { team: TeamResponse }) {
  const updateTeam = useUpdateTeam(team.id);
  const hasAdminPermissions = useIsCurrentMemberTeamAdmin();
  const { teams } = useTeams();
  const teamMembers = useTeamMembers(team.id);
  const projects = useProjects(team.id);
  const [showDeleteTeamModal, setShowDeleteTeamModal] = useState(false);
  const { subscription } = useTeamOrbSubscription(team.id);

  const deleteTeam = useDeleteTeam(team.id);
  const deleteTeamAndRedirect = useCallback(async () => {
    await deleteTeam();
    // Completely reload the page to avoid race conditions
    window.location.href = "/";
  }, [deleteTeam]);

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
          Permanently deletes this team.{" "}
          {!team.managedBy && (
            <>
              To delete your team, you must first remove all team members and
              delete all projects associated with the team.
            </>
          )}
        </p>
        {subscription && (
          <p className="mb-4">
            Deleting your team will automatically cancel your{" "}
            <span className="font-semibold">{subscription.plan.name}</span>{" "}
            subscription.
          </p>
        )}
        {team.managedBy && (
          <div className="flex items-center justify-between gap-4">
            <div>
              This team is managed by {startCase(team.managedBy)}. You may
              delete this Convex team by deleting your Convex integration in{" "}
              {startCase(team.managedBy)}.
            </div>
            <OpenInVercel team={team} />
          </div>
        )}
        {!team.managedBy && (
          <Button
            variant="danger"
            onClick={() => setShowDeleteTeamModal(true)}
            disabled={
              !!team.managedBy ||
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
        )}
        {showDeleteTeamModal && (
          <ConfirmationDialog
            onClose={() => setShowDeleteTeamModal(false)}
            onConfirm={deleteTeamAndRedirect}
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
