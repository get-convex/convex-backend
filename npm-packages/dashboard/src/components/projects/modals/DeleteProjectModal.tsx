import { ExclamationTriangleIcon } from "@radix-ui/react-icons";
import { Callout } from "@ui/Callout";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { Checkbox } from "@ui/Checkbox";
import { LoadingTransition } from "@ui/Loading";
import { WaitForDeploymentApi } from "@common/lib/deploymentContext";
import { useDeployments } from "api/deployments";
import { useTeamMembers } from "api/teams";
import { useDeleteProject } from "api/projects";
import sum from "lodash/sum";
import { useQuery } from "convex/react";
import { ProjectDetails, Team } from "generatedApi";
import udfs from "@common/udfs";
import { useUsageTeamDailyCallsByTag } from "hooks/usageMetrics";
import { isoDateString } from "elements/UsagePeriodSelector";
import { useState } from "react";
import { DeploymentInfoProvider } from "providers/DeploymentInfoProvider";
import { MaybeDeploymentApiProvider } from "providers/MaybeDeploymentApiProvider";

export function DeleteProjectModal({
  onClose,
  onDelete,
  team,
  project,
}: {
  onClose: () => void;
  onDelete?: () => void;
  team: Team;
  project: ProjectDetails;
}) {
  const deleteProject = useDeleteProject(
    team.id,
    project.id,
    project.name || "Untitled Project",
  );

  const handleDelete = async () => {
    onDelete && onDelete();
    await deleteProject();
    onClose();
  };
  const { deployments } = useDeployments(project.id);
  const prodDeployment = deployments?.find((d) => d.deploymentType === "prod");

  return deployments && prodDeployment ? (
    <DeploymentInfoProvider deploymentOverride={prodDeployment.name}>
      <MaybeDeploymentApiProvider deploymentOverride={prodDeployment.name}>
        <WaitForDeploymentApi sizeClass="hidden">
          <DeleteProjectModalContentWithProd
            team={team}
            project={project}
            onClose={onClose}
            handleDelete={handleDelete}
          />
        </WaitForDeploymentApi>
      </MaybeDeploymentApiProvider>
    </DeploymentInfoProvider>
  ) : (
    <DeleteProjectDialog
      onClose={onClose}
      onConfirm={handleDelete}
      validationText={project.isDemo ? undefined : project.name}
    >
      <DeleteProjectModalContent team={team} />
    </DeleteProjectDialog>
  );
}

function DeleteProjectModalContentWithProd({
  team,
  project,
  onClose,
  handleDelete,
}: {
  team: Team;
  project: ProjectDetails;
  onClose: () => void;
  handleDelete: () => Promise<void>;
}) {
  const numFiles = useQuery(udfs.fileStorageV2.numFiles, {
    componentId: null,
  });
  const numDocuments = useQuery(udfs.tableSize.sizeOfAllTables, {
    componentId: null,
  });
  // TODO(nents): Show the number of configured components.
  const lastDay = {
    from: isoDateString(new Date(new Date().getTime() - 24 * 60 * 60 * 1000)),
    to: isoDateString(new Date()),
  };
  const functionCalls = useUsageTeamDailyCallsByTag(
    team.id,
    project.id,
    lastDay,
    null,
  );
  const numFunctionCalls =
    !!functionCalls && functionCalls.length > 0
      ? sum(functionCalls[0].metrics.map((y) => y.value))
      : 0;

  const doneLoading =
    numFiles !== undefined &&
    numDocuments !== undefined &&
    functionCalls !== undefined;
  const showAdditionalConfirmation =
    !doneLoading || numFiles > 0 || numDocuments > 0 || numFunctionCalls > 0;

  const [acceptedConsequences, setAcceptedConsequences] = useState(false);

  const shouldDisable = !acceptedConsequences && showAdditionalConfirmation;

  return (
    <DeleteProjectDialog
      onClose={onClose}
      onConfirm={handleDelete}
      validationText={shouldDisable ? undefined : project.name}
      disableConfirm={shouldDisable}
    >
      <LoadingTransition
        loadingProps={{ fullHeight: false, className: "h-64" }}
      >
        {doneLoading && (
          <DeleteProjectModalContent
            team={team}
            additionalBody={
              showAdditionalConfirmation && (
                <Callout className="flex flex-col gap-2">
                  <div className="flex items-start gap-2">
                    <ExclamationTriangleIcon className="mt-1" />
                    <div className="flex flex-col gap-1">
                      <div className="flex flex-col gap-1">
                        <span>
                          This project is serving{" "}
                          <span className="font-semibold">Production</span>{" "}
                          traffic:
                        </span>
                        <ul className="ml-4 flex list-disc flex-col gap-1">
                          <li>
                            <span className="font-semibold">
                              {numFunctionCalls.toLocaleString()} Function Calls
                            </span>{" "}
                            {functionCalls.length > 0
                              ? `on ${new Date(functionCalls[0].ds).toLocaleDateString()}`
                              : "in the past day"}
                            .
                          </li>
                          <li>
                            <span className="font-semibold">
                              {numDocuments.toLocaleString()} Documents
                            </span>{" "}
                            stored across all tables.
                          </li>
                          <li>
                            <span className="font-semibold">
                              {numFiles.toLocaleString()} Files
                            </span>{" "}
                            stored.
                          </li>
                        </ul>
                      </div>
                    </div>
                  </div>
                  <label className="flex gap-2 text-sm">
                    <Checkbox
                      className="mt-0.5"
                      checked={acceptedConsequences}
                      onChange={(e) =>
                        setAcceptedConsequences(e.currentTarget.checked)
                      }
                    />{" "}
                    By checking this box, I acknowledge the consequences of
                    deleting this project.
                  </label>
                </Callout>
              )
            }
          />
        )}
      </LoadingTransition>
    </DeleteProjectDialog>
  );
}

function DeleteProjectModalContent({
  team,
  additionalBody,
}: {
  team: Team;
  additionalBody?: React.ReactNode;
}) {
  const members = useTeamMembers(team?.id);
  const numOtherTeamMembers = members ? members.length - 1 : 0;

  let warningMessage: React.ReactNode | string =
    "Delete this project and all associated data.";
  if (members && numOtherTeamMembers >= 1) {
    warningMessage = (
      <span>
        Delete this project for you and{" "}
        <span className="font-semibold">
          {numOtherTeamMembers} other team member
          {numOtherTeamMembers > 1 ? "s" : ""}
        </span>
        .
      </span>
    );
  }

  return (
    <div className="flex w-full flex-col gap-2">
      {warningMessage}
      <span className="font-semibold">
        Deleted projects cannot be recovered.
      </span>
      {additionalBody}
    </div>
  );
}

function DeleteProjectDialog({
  children,
  ...props
}: {
  onClose: () => void;
  onConfirm: () => Promise<void>;
  validationText?: string;
  disableConfirm?: boolean;
  children: React.ReactNode;
}) {
  return (
    <ConfirmationDialog
      {...props}
      confirmText="Delete Project"
      variant="danger"
      dialogTitle="Delete Project"
      dialogBody={children}
    />
  );
}
