import { useBBMutation } from "api/api";
import { useDeployments } from "api/deployments";
import { useProfile } from "api/profile";
import { useCurrentProject, useProjects } from "api/projects";
import {
  useTeams,
  useCurrentTeam,
  useTeamMembers,
  useTeamEntitlements,
} from "api/teams";
import { Sheet } from "dashboard-common/elements/Sheet";
import { Combobox } from "dashboard-common/elements/Combobox";
import { Button } from "dashboard-common/elements/Button";
import { Spinner } from "dashboard-common/elements/Spinner";
import { Callout } from "dashboard-common/elements/Callout";
import { ConfirmationDialog } from "dashboard-common/elements/ConfirmationDialog";
import { useRouter } from "next/router";
import { useState } from "react";

function useTransferProject(projectId?: number, destinationTeamId?: number) {
  return useBBMutation({
    path: "/projects/{project_id}/transfer",
    pathParams: {
      project_id: projectId?.toString() || "",
    },
    mutateKey: "/teams/{team_id}/projects",
    mutatePathParams: {
      team_id: destinationTeamId?.toString() || "",
    },
    successToast: "Project transferred.",
  });
}

export function TransferProject() {
  const project = useCurrentProject();
  const { selectedTeamSlug, teams } = useTeams();
  const originTeam = useCurrentTeam();
  const [destinationTeamId, setDestinationTeamId] = useState<number | null>(
    null,
  );
  const transferProject = useTransferProject(
    project?.id,
    destinationTeamId ?? undefined,
  );
  const destinationTeam = teams?.find((t) => t.id === destinationTeamId);

  const me = useProfile();
  const originTeamMembers = useTeamMembers(originTeam?.id);
  const destinationTeamMembers = useTeamMembers(destinationTeamId ?? undefined);
  const isAdminOfOldTeam =
    originTeamMembers?.find((member) => member.id === me?.id)?.role === "admin";
  const isAdminOfNewTeam =
    destinationTeamMembers?.find((member) => member.id === me?.id)?.role ===
    "admin";

  const membersNotOnNewTeam = originTeamMembers?.filter(
    (member) => !destinationTeamMembers?.some((m) => m.id === member.id),
  );

  const entitlements = useTeamEntitlements(destinationTeamId ?? undefined);
  const maxProjects = entitlements?.maxProjects ?? 0;
  const destinationTeamProjects = useProjects(
    destinationTeamId ?? undefined,
  )?.filter((p) => !p.isDemo);

  const overProjectLimit =
    destinationTeamProjects && destinationTeamProjects.length >= maxProjects;

  const { deployments } = useDeployments(project?.id);

  const deploymentsToBeDeleted = deployments?.filter((deployment) =>
    membersNotOnNewTeam?.some((member) => member.id === deployment.creator),
  );

  const loading = destinationTeamId
    ? !originTeamMembers ||
      !destinationTeamMembers ||
      !entitlements ||
      !destinationTeamProjects
    : false;
  const canTransfer = isAdminOfOldTeam && isAdminOfNewTeam;

  const [showConfirmation, setShowConfirmation] = useState(false);
  const validationError = !destinationTeamId
    ? undefined
    : overProjectLimit
      ? `${destinationTeam?.name} has reached it's project limit of ${maxProjects}.`
      : teams && teams.length === 1
        ? "You must be a member of another team to transfer a project."
        : !canTransfer
          ? `You must be an admin of ${originTeam?.name} and ${destinationTeam?.name} to transfer this project to ${destinationTeam?.name}.`
          : undefined;
  const router = useRouter();

  return (
    <Sheet>
      <h3 className="mb-4">Transfer Project</h3>
      <p className="mb-5  max-w-prose text-sm text-content-primary">
        Transfer this project to another team.
      </p>
      {teams && teams.length > 1 && (
        <div className="mb-4 flex flex-col gap-1">
          <Combobox
            label={
              <div className="flex items-center gap-2">
                Destination Team
                {loading && (
                  <Spinner className="ml-0 animate-fadeInFromLoading opacity-50" />
                )}
              </div>
            }
            labelHidden={false}
            placeholder="Select a team"
            buttonProps={{
              tip:
                validationError ||
                (!isAdminOfOldTeam &&
                  "You must be an admin of this team to transfer a project."),
            }}
            options={
              teams
                ?.filter((t) => t.slug !== selectedTeamSlug)
                .map((team) => ({
                  label: team.name,
                  value: team.id,
                })) || []
            }
            selectedOption={destinationTeamId}
            setSelectedOption={setDestinationTeamId}
            disabled={!isAdminOfOldTeam || !teams || teams.length === 1}
          />
          {!loading && validationError && (
            <p
              className="max-w-prose animate-fadeInFromLoading text-xs text-content-errorSecondary"
              role="alert"
            >
              {validationError}
            </p>
          )}
        </div>
      )}
      <Button
        variant="primary"
        disabled={
          loading ||
          !destinationTeamId ||
          overProjectLimit ||
          !canTransfer ||
          (teams && teams.length === 1)
        }
        tip={
          teams && teams.length === 1
            ? "You must be a member of another team to transfer a project."
            : !destinationTeamId
              ? "Select a team to transfer this project to."
              : undefined
        }
        onClick={() => setShowConfirmation(true)}
      >
        Transfer
      </Button>
      {project &&
        originTeam &&
        destinationTeam &&
        project &&
        showConfirmation && (
          <ConfirmationDialog
            confirmText="Transfer"
            validationText={`Transfer ${project.slug} from ${originTeam.slug} to ${destinationTeam.slug}`}
            dialogTitle={`Transfer Project to ${destinationTeam.name}?`}
            dialogBody={
              <div className="flex flex-col gap-2">
                Are you sure you want to transfer this project?
                {deploymentsToBeDeleted &&
                  deploymentsToBeDeleted.length > 0 && (
                    <Callout className="block">
                      {deploymentsToBeDeleted.length} development deployment
                      {deploymentsToBeDeleted.length > 1 ? "s" : ""} will be
                      deleted because their creators are not members of{" "}
                      <span className="font-semibold">
                        {destinationTeam.name}
                      </span>
                      .
                    </Callout>
                  )}
              </div>
            }
            onConfirm={async () => {
              await transferProject({
                destinationTeamId: destinationTeam.id,
              });
              setShowConfirmation(false);
              await router.replace(
                `/t/${destinationTeam.slug}/${project.slug}/settings`,
              );
            }}
            variant="primary"
            onClose={() => setShowConfirmation(false)}
          />
        )}
    </Sheet>
  );
}
